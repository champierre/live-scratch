(function () {
    'use strict';

    var POLL_INTERVAL = 100;
    var RECONNECT_DELAY = 2000;
    var SAVE_DEBOUNCE = 1000;
    var wsUrl = 'ws://' + location.host + '/';
    var ws = null;
    var indicator = null;
    var ignoreChanges = false;
    var saveTimer = null;

    function createIndicator() {
        indicator = document.createElement('div');
        indicator.style.cssText =
            'position:fixed;top:8px;right:8px;width:12px;height:12px;' +
            'border-radius:50%;background:#e44;z-index:999999;' +
            'transition:background 0.3s;pointer-events:none;';
        document.body.appendChild(indicator);
    }

    function setConnected(connected) {
        if (indicator) {
            indicator.style.background = connected ? '#4c4' : '#e44';
        }
    }

    function connect() {
        ws = new WebSocket(wsUrl);
        ws.binaryType = 'arraybuffer';

        ws.onopen = function () {
            console.log('[live-scratch] connected');
            setConnected(true);
        };

        ws.onmessage = function (event) {
            if (!(event.data instanceof ArrayBuffer)) return;
            console.log('[live-scratch] reloading project (' + event.data.byteLength + ' bytes)');

            var editingTarget = window.vm.editingTarget ?
                window.vm.editingTarget.id : null;

            ignoreChanges = true;
            if (saveTimer) {
                clearTimeout(saveTimer);
                saveTimer = null;
            }

            window.vm.loadProject(event.data).then(function () {
                console.log('[live-scratch] project loaded');
                if (editingTarget) {
                    try {
                        window.vm.setEditingTarget(editingTarget);
                    } catch (e) {
                        // target may no longer exist
                    }
                }
                setTimeout(function () { ignoreChanges = false; }, 500);
            }).catch(function (err) {
                console.error('[live-scratch] load error:', err);
                ignoreChanges = false;
            });
        };

        ws.onclose = function () {
            console.log('[live-scratch] disconnected, reconnecting in ' + RECONNECT_DELAY + 'ms');
            setConnected(false);
            setTimeout(connect, RECONNECT_DELAY);
        };

        ws.onerror = function () {
            ws.close();
        };

        // Listen for project changes made in the Scratch editor
        window.vm.on('PROJECT_CHANGED', function () {
            if (ignoreChanges) return;
            if (saveTimer) clearTimeout(saveTimer);
            saveTimer = setTimeout(function () {
                saveTimer = null;
                if (ignoreChanges) return;
                if (!ws || ws.readyState !== 1) return;

                console.log('[live-scratch] saving project to server');
                window.vm.saveProjectSb3().then(function (blob) {
                    return blob.arrayBuffer();
                }).then(function (buffer) {
                    if (ws && ws.readyState === 1) {
                        ws.send(buffer);
                        console.log('[live-scratch] sent sb3 (' + buffer.byteLength + ' bytes)');
                    }
                }).catch(function (err) {
                    console.error('[live-scratch] save error:', err);
                });
            }, SAVE_DEBOUNCE);
        });
    }

    function waitForVM() {
        if (window.vm) {
            createIndicator();
            connect();
        } else {
            setTimeout(waitForVM, POLL_INTERVAL);
        }
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', waitForVM);
    } else {
        waitForVM();
    }
})();
