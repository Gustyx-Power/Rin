package com.rin.ui.screen

import android.view.View
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.rin.RinLib
import com.rin.terminal.SessionManager
import com.rin.ui.components.ExtraKeysBar
import com.rin.ui.components.SessionDialog
import com.rin.ui.components.TerminalSurface

@Composable
fun TerminalScreen(
    sessionManager: SessionManager,
    modifier: Modifier = Modifier
) {
    var ctrlPressed by remember { mutableStateOf(false) }
    var keyRepeating by remember { mutableStateOf(false) }
    var terminalView by remember { mutableStateOf<View?>(null) }
    var showSessionDialog by remember { mutableStateOf(false) }

    val activeSession = sessionManager.activeSession
    val engineHandle = activeSession?.engineHandle ?: 0L

    Column(
        modifier = modifier
            .fillMaxSize()
            .systemBarsPadding()
            .imePadding()
    ) {
        TerminalSurface(
            engineHandle = engineHandle,
            ctrlPressed = ctrlPressed,
            cursorBlinkEnabled = !keyRepeating,
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
            onInput = { data ->
                if (engineHandle != 0L) {
                    RinLib.write(engineHandle, data)
                }
            },
            onViewReady = { view -> terminalView = view }
        )

        ExtraKeysBar(
            onKeyPress = { code ->
                if (engineHandle != 0L) {
                    RinLib.write(engineHandle, code.toByteArray())
                    terminalView?.invalidate()
                }
            },
            onCtrlToggle = { active ->
                ctrlPressed = active
            },
            onRepeatStateChange = { repeating ->
                keyRepeating = repeating
            },
            sessionName = activeSession?.name ?: "No Session",
            onSessionButtonClick = { showSessionDialog = true },
            modifier = Modifier.fillMaxWidth()
        )
    }

    if (showSessionDialog) {
        SessionDialog(
            sessions = sessionManager.sessions,
            activeIndex = sessionManager.activeIndex,
            onDismiss = { showSessionDialog = false },
            onSwitchSession = { index ->
                sessionManager.switchSession(index)
            },
            onCreateSession = {
                sessionManager.createSession()
            },
            onRemoveSession = { index ->
                sessionManager.removeSession(index)
            },
            onRenameSession = { index, newName ->
                sessionManager.renameSession(index, newName)
            }
        )
    }
}
