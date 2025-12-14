#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');

const binPath = path.join(__dirname, '..', 'bin', 'ghgrab' + (process.platform === 'win32' ? '.exe' : ''));

const child = spawn(binPath, process.argv.slice(2), {
    stdio: 'inherit',
    windowsHide: true
});

child.on('exit', (code) => {
    process.exit(code);
});
