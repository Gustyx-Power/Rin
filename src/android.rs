#[cfg(feature = "android")]
use crate::{renderer::AndroidRenderer, Pty, TerminalEngine};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jint, jlong};
use jni::EnvUnowned;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread;

type EngineHandle = jlong;

struct AndroidSession {
    engine: Arc<Mutex<TerminalEngine>>,
    pty: Arc<Mutex<Pty>>,
    // We keep these to ensure they live as long as the session
    // reader_thread: Option<thread::JoinHandle<()>>,
}

static SESSIONS: OnceLock<Arc<RwLock<HashMap<EngineHandle, AndroidSession>>>> = OnceLock::new();
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn get_sessions() -> Arc<RwLock<HashMap<EngineHandle, AndroidSession>>> {
    SESSIONS
        .get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
        .clone()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_createEngine(
    mut env: EnvUnowned,
    _class: JClass,
    width: jint,
    height: jint,
    font_size: f32,
    home_dir: JString,
    username: JString,
) -> jlong {
    #[cfg(feature = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("RinNative"),
    );

    let home_dir_str: String = env
        .with_env(|env| -> jni::errors::Result<String> {
            let jstr: jni::objects::JString = home_dir;
            #[allow(deprecated)]
            let java_str = env.get_string(&jstr).map(|s| String::from(s))?;
            Ok(java_str)
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();

    let mut username_str: String = env
        .with_env(|env| -> jni::errors::Result<String> {
            let jstr: jni::objects::JString = username;
            #[allow(deprecated)]
            let java_str = env.get_string(&jstr).map(|s| String::from(s))?;
            Ok(java_str)
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();

    if username_str.is_empty() {
        username_str = "user".to_string();
    }

    log::info!(
        "Creating Engine: {}x{}, HOME={}, USER={}",
        width,
        height,
        home_dir_str,
        username_str
    );

    // 1. Create Renderer & Engine
    let renderer = Box::new(AndroidRenderer::new(font_size));
    let engine = Arc::new(Mutex::new(TerminalEngine::new(
        width as usize,
        height as usize,
        renderer,
    )));

    // 2. Write startup banner
    {
        let mut engine_guard = engine.lock().unwrap();
        let banner = concat!(
            "\x1b[36m",
            r"  ____  _       ",
            "\r\n",
            r" |  _ \(_)_ __  ",
            "\r\n",
            r" | |_) | | '_ \ ",
            "\r\n",
            r" |  _ <| | | | |",
            "\r\n",
            r" |_| \_\_|_| |_|",
            "\r\n",
            "\x1b[0m\r\n",
            " \x1b[90mTerminal v",
            env!("CARGO_PKG_VERSION"),
            "\x1b[0m\r\n",
            " \x1b[90mgithub.com/pavelc4/Rin\x1b[0m\r\n",
            "\r\n",
        );
        let _ = engine_guard.write(banner.as_bytes());
    }

    // 3. Spawn PTY with home directory and username
    let pty = match Pty::spawn(
        "/system/bin/sh",
        width as u16,
        height as u16,
        Some(&home_dir_str),
        Some(&username_str),
    ) {
        Ok(pty) => Arc::new(Mutex::new(pty)),
        Err(e) => {
            log::error!("Failed to spawn PTY: {}", e);
            return -1;
        }
    };

    // 3. Spawn Reader Thread (PTY -> Engine)
    let pty_clone = pty.clone();
    let engine_clone = engine.clone();

    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        let mut reader = {
            let mut pty_guard = pty_clone.lock().unwrap();
            match pty_guard.take_reader() {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to take PTY reader: {}", e);
                    return;
                }
            }
        };

        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    log::info!("PTY closed (EOF)");
                    break;
                }
                Ok(n) => {
                    let mut engine_guard = engine_clone.lock().unwrap();
                    if let Err(e) = engine_guard.write(&buffer[..n]) {
                        log::error!("Failed to write to engine: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("Error reading from PTY: {}", e);
                    break;
                }
            }
        }
    });

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let session = AndroidSession { engine, pty };

    let sessions_arc = get_sessions();
    sessions_arc.write().unwrap().insert(handle, session);

    log::info!("Engine created with handle: {}", handle);
    handle
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_destroyEngine(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
) {
    let sessions_arc = get_sessions();
    sessions_arc.write().unwrap().remove(&handle);
    log::info!("Engine destroyed: {}", handle);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_write(
    mut env: EnvUnowned,
    _class: JClass,
    handle: jlong,
    data: JByteArray,
) -> jint {
    let outcome = env.with_env(|env| env.convert_byte_array(&data));
    let bytes: Vec<u8> = outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    let bytes: &[u8] = &bytes;
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        // Write to PTY, not Engine
        let mut pty = session.pty.lock().unwrap();
        match pty.write(&bytes) {
            Ok(_) => 0,
            Err(e) => {
                log::error!("Failed to write to PTY: {}", e);
                -1
            }
        }
    } else {
        -2 // Handle not found
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_render(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
) -> jint {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let mut engine = session.engine.lock().unwrap();
        match engine.render() {
            Ok(_) => 0,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_resize(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
    width: jint,
    height: jint,
) -> jint {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        // Resize both Engine and PTY
        let mut engine = session.engine.lock().unwrap();
        let _ = engine.resize(width as usize, height as usize);

        let mut pty = session.pty.lock().unwrap();
        let _ = pty.resize(width as u16, height as u16);
        0
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_getLine<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    y: jint,
) -> JString<'local> {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let engine = session.engine.lock().unwrap();
        let buffer = engine.buffer();
        let grid = buffer.grid();
        if let Some(row) = grid.row(y as usize) {
            let line: String = row.iter().map(|c| c.character).collect();
            return env
                .with_env(|env| env.new_string(line))
                .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
        }
    }
    env.with_env(|env| env.new_string(""))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_getCursorX(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
) -> jint {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let engine = session.engine.lock().unwrap();
        engine.buffer().cursor_pos().0 as jint
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_getCursorY(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
) -> jint {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let engine = session.engine.lock().unwrap();
        engine.buffer().cursor_pos().1 as jint
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_getCellData<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    handle: jlong,
    y: jint,
) -> JString<'local> {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let engine = session.engine.lock().unwrap();
        let buffer = engine.buffer();
        let grid = buffer.grid();
        if let Some(row) = grid.row(y as usize) {
            let mut result = String::with_capacity(row.len() * 32);
            for cell in row.iter() {
                // Skip wide spacer cells
                if cell.wide_spacer {
                    continue;
                }

                let style = &cell.style;
                let (fg, bg) = if style.reverse {
                    (&style.bg, &style.fg)
                } else {
                    (&style.fg, &style.bg)
                };

                // Format: char\tfgR,fgG,fgB\tbgR,bgG,bgB\tflags (tab-separated)
                // Use write! instead of format! to avoid heap allocations
                let _ = write!(
                    result,
                    "{}\t{},{},{}\t{},{},{}",
                    cell.character, fg.r, fg.g, fg.b, bg.r, bg.g, bg.b
                );
                result.push('\t');

                // Flags
                if style.bold {
                    result.push('b');
                }
                if style.italic {
                    result.push('i');
                }
                if style.dim {
                    result.push('d');
                }
                if cell.wide {
                    result.push('w');
                }

                result.push('\n');
            }
            return env
                .with_env(|env| env.new_string(result))
                .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
        }
    }
    env.with_env(|env| env.new_string(""))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_hasDirtyRows(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
) -> bool {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let engine = session.engine.lock().unwrap();
        engine.buffer().grid().has_dirty_rows()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rin_RinLib_clearDirty(
    _env: EnvUnowned,
    _class: JClass,
    handle: jlong,
) {
    let sessions_arc = get_sessions();
    let sessions = sessions_arc.read().unwrap();
    if let Some(session) = sessions.get(&handle) {
        let mut engine = session.engine.lock().unwrap();
        engine.buffer_mut().grid_mut().clear_dirty();
    }
}
