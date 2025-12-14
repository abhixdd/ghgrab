const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const version = require('../package.json').version;
const binDir = path.join(__dirname, '..', 'bin');

function getPlatform() {
    const platform = process.platform;
    const arch = process.arch;

    if (platform === 'win32') return 'win32';
    if (platform === 'darwin' && arch === 'arm64') return 'darwin-arm64';
    if (platform === 'darwin') return 'darwin';
    if (platform === 'linux') return 'linux';

    throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function getBinaryName() {
    return 'ghgrab' + (process.platform === 'win32' ? '.exe' : '');
}

function download(url, dest) {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(dest);
        https.get(url, (response) => {
            if (response.statusCode === 302 || response.statusCode === 301) {
                return download(response.headers.location, dest).then(resolve).catch(reject);
            }
            response.pipe(file);
            file.on('finish', () => {
                file.close();
                resolve();
            });
        }).on('error', (err) => {
            fs.unlink(dest, () => { });
            reject(err);
        });
    });
}

async function install() {
    try {
        const platformName = getPlatform();
        const binaryName = getBinaryName();
        const downloadUrl = `https://github.com/abhixdd/ghgrab/releases/download/v${version}/ghgrab-${platformName}`;

        if (!fs.existsSync(binDir)) {
            fs.mkdirSync(binDir, { recursive: true });
        }

        const binPath = path.join(binDir, binaryName);

        console.log(`Downloading ghgrab binary for ${platformName}...`);
        await download(downloadUrl, binPath);

        if (process.platform !== 'win32') {
            fs.chmodSync(binPath, 0o755);
        }

        console.log('ghgrab installed successfully!');
    } catch (error) {
        console.error('Failed to download binary:', error.message);
        console.log('\nFalling back to building from source...');

        try {
            execSync('cargo build --release', {
                cwd: path.join(__dirname, '..'),
                stdio: 'inherit'
            });

            const sourceBin = path.join(__dirname, '..', 'target', 'release', getBinaryName());
            const targetBin = path.join(binDir, getBinaryName());

            if (!fs.existsSync(binDir)) {
                fs.mkdirSync(binDir, { recursive: true });
            }

            fs.copyFileSync(sourceBin, targetBin);

            if (process.platform !== 'win32') {
                fs.chmodSync(targetBin, 0o755);
            }

            console.log('Built from source successfully!');
        } catch (buildError) {
            console.error('Build from source also failed:', buildError.message);
            process.exit(1);
        }
    }
}

install();
