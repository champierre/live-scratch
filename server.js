#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const http = require('http');
const express = require('express');
const {WebSocketServer} = require('ws');
const chokidar = require('chokidar');
const JSZip = require('jszip');

// --- CLI args ---
const args = process.argv.slice(2);
let sb3Path = null;
let port = 3333;

for (let i = 0; i < args.length; i++) {
    if (args[i] === '--port' && args[i + 1]) {
        port = parseInt(args[i + 1], 10);
        i++;
    } else if (!args[i].startsWith('-')) {
        sb3Path = args[i];
    }
}

if (sb3Path) {
    sb3Path = path.resolve(sb3Path);
    if (!fs.existsSync(sb3Path)) {
        console.error(`File not found: ${sb3Path}`);
        process.exit(1);
    }
}

const WORKSPACE = path.join(process.cwd(), 'workspace');
const DEFAULT_PROJECT = path.join(__dirname, 'default-project');
const BUILD_DIR = path.join(__dirname, 'scratch-editor', 'packages', 'scratch-gui', 'build');
const CLIENT_DIR = path.join(__dirname, 'client');

// --- Extract sb3 to workspace ---
async function extractSb3FromData(data) {
    const zip = await JSZip.loadAsync(data);

    fs.mkdirSync(WORKSPACE, {recursive: true});

    const entries = Object.keys(zip.files);
    for (const name of entries) {
        const entry = zip.files[name];
        if (entry.dir) continue;

        const outPath = path.join(WORKSPACE, name);
        const content = await entry.async('nodebuffer');

        if (name === 'project.json') {
            // Pretty-print project.json
            try {
                const json = JSON.parse(content.toString('utf8'));
                fs.writeFileSync(outPath, JSON.stringify(json, null, 2));
            } catch {
                fs.writeFileSync(outPath, content);
            }
        } else {
            fs.writeFileSync(outPath, content);
        }
    }

    console.log(`Extracted ${entries.length} files to workspace/`);
}

async function extractSb3(filePath) {
    const data = fs.readFileSync(filePath);
    await extractSb3FromData(data);
}

// --- Build sb3 from workspace ---
async function buildSb3() {
    const zip = new JSZip();
    const files = fs.readdirSync(WORKSPACE);

    for (const file of files) {
        const filePath = path.join(WORKSPACE, file);
        const stat = fs.statSync(filePath);
        if (!stat.isFile()) continue;

        const content = fs.readFileSync(filePath);

        if (file === 'project.json') {
            // Validate JSON
            try {
                JSON.parse(content.toString('utf8'));
            } catch (err) {
                console.error(`[live-scratch] JSON syntax error in project.json: ${err.message}`);
                return null;
            }
        }

        zip.file(file, content);
    }

    return zip.generateAsync({type: 'arraybuffer', compression: 'STORE'});
}

// --- Copy default project to workspace ---
function copyDefaultProject() {
    fs.mkdirSync(WORKSPACE, {recursive: true});
    const files = fs.readdirSync(DEFAULT_PROJECT);
    for (const file of files) {
        fs.copyFileSync(path.join(DEFAULT_PROJECT, file), path.join(WORKSPACE, file));
    }
    console.log(`Copied ${files.length} files from default-project/ to workspace/`);
}

// --- Server setup ---
async function start() {
    if (sb3Path) {
        await extractSb3(sb3Path);
    } else if (!fs.existsSync(path.join(WORKSPACE, 'project.json'))) {
        copyDefaultProject();
    }

    // Build initial sb3
    let currentSb3 = await buildSb3();

    const app = express();

    // Serve live-reload.js
    app.get('/live-reload.js', (req, res) => {
        res.sendFile(path.join(CLIENT_DIR, 'live-reload.js'));
    });

    // Serve scratch-gui build with script injection
    app.get('/', (req, res) => {
        const indexPath = path.join(BUILD_DIR, 'index.html');
        let html = fs.readFileSync(indexPath, 'utf8');
        html = html.replace('</body>', '<script src="/live-reload.js"></script></body>');
        res.type('html').send(html);
    });

    // Static files from scratch-gui build
    app.use(express.static(BUILD_DIR));

    const server = http.createServer(app);
    const wss = new WebSocketServer({server});

    // --- File change ignore flag (for bi-directional sync loop prevention) ---
    let ignoreFileChange = false;

    wss.on('connection', (ws) => {
        console.log('[live-scratch] client connected');
        if (currentSb3) {
            ws.send(currentSb3);
        }

        ws.on('message', async (data) => {
            if (!(data instanceof Buffer) || data.length === 0) return;
            console.log(`[live-scratch] received sb3 from client (${data.length} bytes)`);

            ignoreFileChange = true;
            try {
                await extractSb3FromData(data);
                currentSb3 = await buildSb3();

                // Broadcast to other clients
                let sent = 0;
                for (const client of wss.clients) {
                    if (client !== ws && client.readyState === 1) {
                        client.send(currentSb3);
                        sent++;
                    }
                }
                if (sent > 0) {
                    console.log(`[live-scratch] broadcast to ${sent} other client(s)`);
                }
            } catch (err) {
                console.error('[live-scratch] error processing client sb3:', err);
            }
            setTimeout(() => { ignoreFileChange = false; }, 1000);
        });
    });

    // --- File watcher ---
    let debounceTimer = null;

    const watcher = chokidar.watch(WORKSPACE, {
        ignoreInitial: true,
        awaitWriteFinish: {
            stabilityThreshold: 300,
            pollInterval: 100
        }
    });

    async function onFileChange(filePath) {
        if (ignoreFileChange) return;
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(async () => {
            const relPath = path.relative(WORKSPACE, filePath);
            console.log(`[live-scratch] file changed: ${relPath}`);

            const sb3 = await buildSb3();
            if (!sb3) {
                console.log('[live-scratch] skipping send (build error)');
                return;
            }

            currentSb3 = sb3;
            let sent = 0;
            for (const client of wss.clients) {
                if (client.readyState === 1) { // WebSocket.OPEN
                    client.send(sb3);
                    sent++;
                }
            }
            console.log(`[live-scratch] sent to ${sent} client(s)`);
        }, 200);
    }

    watcher.on('change', onFileChange);
    watcher.on('add', onFileChange);
    watcher.on('unlink', onFileChange);

    server.listen(port, async () => {
        console.log(`[live-scratch] server running at http://localhost:${port}`);
        try {
            const open = (await import('open')).default;
            open(`http://localhost:${port}`);
        } catch {
            // open is optional
        }
    });
}

start().catch(err => {
    console.error('Failed to start:', err);
    process.exit(1);
});
