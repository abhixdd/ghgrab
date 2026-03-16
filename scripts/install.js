const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const crypto = require('crypto');

const version = require('../package.json').version;
const binDir = path.join(__dirname, '..', 'bin');

function getPlatformInfo() {
    const platform = process.platform;
    const arch = process.arch;

    if (platform === 'win32') {
        return { platformName: 'win32', binaryName: 'ghgrab-win32.exe', outName: 'ghgrab.exe' };
    }
    if (platform === 'darwin' && arch === 'arm64') {
        return { platformName: 'darwin-arm64', binaryName: 'ghgrab-darwin-arm64', outName: 'ghgrab' };
    }
    if (platform === 'darwin') {
        return { platformName: 'darwin', binaryName: 'ghgrab-darwin', outName: 'ghgrab' };
    }
    if (platform === 'linux' && arch === 'arm64') {
        return { platformName: 'linux-arm64', binaryName: 'ghgrab-linux-arm64', outName: 'ghgrab' };
    }
    if (platform === 'linux') {
        return { platformName: 'linux', binaryName: 'ghgrab-linux', outName: 'ghgrab' };
    }

    throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function download(url, dest, redirectCount = 0) {
    if (redirectCount > 10) return Promise.reject(new Error('Too many redirects'));
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(dest);
        https.get(url, { headers: { 'User-Agent': 'ghgrab-npm-installer' } }, (response) => {
            if (response.statusCode === 301 || response.statusCode === 302 || response.statusCode === 307 || response.statusCode === 308) {
                file.close();
                fs.unlink(dest, () => {});
                return download(response.headers.location, dest, redirectCount + 1).then(resolve).catch(reject);
            }
            if (response.statusCode !== 200) {
                file.close();
                fs.unlink(dest, () => {});
                return reject(new Error(`HTTP ${response.statusCode} downloading from ${url}`));
            }
            response.pipe(file);
            file.on('finish', () => {
                file.close(() => resolve());
            });
        }).on('error', (err) => {
            fs.unlink(dest, () => {});
            reject(err);
        });
    });
}

async function install() {
    const { platformName, binaryName, outName } = getPlatformInfo();
    const downloadUrl = `https://github.com/abhixdd/ghgrab/releases/download/v${version}/${binaryName}`;
    const binPath = path.join(binDir, outName);

    if (!fs.existsSync(binDir)) {
        fs.mkdirSync(binDir, { recursive: true });
    }

    console.log(`Downloading ghgrab v${version} for ${platformName}...`);
    console.log(`  URL: ${downloadUrl}`);

    try {
        await download(downloadUrl, binPath);

        // Verify it's not empty / HTML error page
        const stat = fs.statSync(binPath);
        if (stat.size < 100000) {
            throw new Error(`Downloaded file is too small (${stat.size} bytes) — likely not a valid binary`);
        }

        const checksumFile = path.join(__dirname, '..', 'checksums.json');
        if (fs.existsSync(checksumFile)) {
            const checksums = JSON.parse(fs.readFileSync(checksumFile, 'utf8'));
            const expectedHash = checksums[binaryName];
            if (expectedHash) {
                const fileBuffer = fs.readFileSync(binPath);
                const hashSum = crypto.createHash('sha256');
                hashSum.update(fileBuffer);
                const actualHash = hashSum.digest('hex');
                if (actualHash !== expectedHash) {
                    throw new Error(`Checksum mismatch for ${binaryName}. Expected ${expectedHash}, got ${actualHash}.`);
                }
                console.log(`✓ Checksum verified for ${binaryName}`);
            }
        }

        if (process.platform !== 'win32') {
            fs.chmodSync(binPath, 0o755);
        }

        console.log(`✓ ghgrab installed successfully to ${binPath}`);
    } catch (downloadErr) {
        console.error(`\nDownload failed: ${downloadErr.message}`);
        console.log('\nFalling back to building from source (requires Rust/cargo)...');

        try {
            execSync('cargo build --release', {
                cwd: path.join(__dirname, '..'),
                stdio: 'inherit'
            });

            const builtBin = process.platform === 'win32' ? 'ghgrab.exe' : 'ghgrab';
            const sourceBin = path.join(__dirname, '..', 'target', 'release', builtBin);
            fs.copyFileSync(sourceBin, binPath);

            if (process.platform !== 'win32') {
                fs.chmodSync(binPath, 0o755);
            }

            console.log(`✓ Built from source successfully!`);
        } catch (buildErr) {
            console.error(`\nCould not build from source: ${buildErr.message}`);
            console.error('Please install Rust from https://rustup.rs and try again, or download manually from:');
            console.error(`  https://github.com/abhixdd/ghgrab/releases`);
            process.exit(1);
        }
    }
}

install();
