/* JavaScript entry point for scap */

const { platform, arch } = process;

let nativeBinding = null;
let localFileExisted = false;

function isMusl() {
  if (!process.report || typeof process.report.getReport !== 'function') {
    try {
      const lddPath = require('child_process').execSync('which ldd').toString().trim();
      return require('fs').readFileSync(lddPath, 'utf8').includes('musl');
    } catch (e) {
      return true;
    }
  } else {
    const { glibcVersionRuntime } = process.report.getReport().header;
    return !glibcVersionRuntime;
  }
}

function loadNativeBinding() {
  const { existsSync } = require('fs');
  const { join } = require('path');

  switch (platform) {
    case 'darwin':
      localFileExisted = existsSync(join(__dirname, 'scap.darwin-universal.node'));
      try {
        if (localFileExisted) {
          nativeBinding = require('./scap.darwin-universal.node');
        } else {
          nativeBinding = require('scap-darwin-universal');
        }
        break;
      } catch {}
      
      switch (arch) {
        case 'x64':
          localFileExisted = existsSync(join(__dirname, 'scap.darwin-x64.node'));
          try {
            if (localFileExisted) {
              nativeBinding = require('./scap.darwin-x64.node');
            } else {
              nativeBinding = require('scap-darwin-x64');
            }
          } catch (e) {
            throw new Error(`Failed to load native binding for macOS x64: ${e.message}`);
          }
          break;
        case 'arm64':
          localFileExisted = existsSync(join(__dirname, 'scap.darwin-arm64.node'));
          try {
            if (localFileExisted) {
              nativeBinding = require('./scap.darwin-arm64.node');
            } else {
              nativeBinding = require('scap-darwin-arm64');
            }
          } catch (e) {
            throw new Error(`Failed to load native binding for macOS arm64: ${e.message}`);
          }
          break;
        default:
          throw new Error(`Unsupported architecture on macOS: ${arch}`);
      }
      break;

    case 'win32':
      switch (arch) {
        case 'x64':
          localFileExisted = existsSync(join(__dirname, 'scap.win32-x64-msvc.node'));
          try {
            if (localFileExisted) {
              nativeBinding = require('./scap.win32-x64-msvc.node');
            } else {
              nativeBinding = require('scap-win32-x64-msvc');
            }
          } catch (e) {
            throw new Error(`Failed to load native binding for Windows x64: ${e.message}`);
          }
          break;
        case 'arm64':
          localFileExisted = existsSync(join(__dirname, 'scap.win32-arm64-msvc.node'));
          try {
            if (localFileExisted) {
              nativeBinding = require('./scap.win32-arm64-msvc.node');
            } else {
              nativeBinding = require('scap-win32-arm64-msvc');
            }
          } catch (e) {
            throw new Error(`Failed to load native binding for Windows arm64: ${e.message}`);
          }
          break;
        default:
          throw new Error(`Unsupported architecture on Windows: ${arch}`);
      }
      break;

    case 'linux':
      switch (arch) {
        case 'x64':
          if (isMusl()) {
            localFileExisted = existsSync(join(__dirname, 'scap.linux-x64-musl.node'));
            try {
              if (localFileExisted) {
                nativeBinding = require('./scap.linux-x64-musl.node');
              } else {
                nativeBinding = require('scap-linux-x64-musl');
              }
            } catch (e) {
              throw new Error(`Failed to load native binding for Linux x64 musl: ${e.message}`);
            }
          } else {
            localFileExisted = existsSync(join(__dirname, 'scap.linux-x64-gnu.node'));
            try {
              if (localFileExisted) {
                nativeBinding = require('./scap.linux-x64-gnu.node');
              } else {
                nativeBinding = require('scap-linux-x64-gnu');
              }
            } catch (e) {
              throw new Error(`Failed to load native binding for Linux x64 gnu: ${e.message}`);
            }
          }
          break;
        case 'arm64':
          if (isMusl()) {
            localFileExisted = existsSync(join(__dirname, 'scap.linux-arm64-musl.node'));
            try {
              if (localFileExisted) {
                nativeBinding = require('./scap.linux-arm64-musl.node');
              } else {
                nativeBinding = require('scap-linux-arm64-musl');
              }
            } catch (e) {
              throw new Error(`Failed to load native binding for Linux arm64 musl: ${e.message}`);
            }
          } else {
            localFileExisted = existsSync(join(__dirname, 'scap.linux-arm64-gnu.node'));
            try {
              if (localFileExisted) {
                nativeBinding = require('./scap.linux-arm64-gnu.node');
              } else {
                nativeBinding = require('scap-linux-arm64-gnu');
              }
            } catch (e) {
              throw new Error(`Failed to load native binding for Linux arm64 gnu: ${e.message}`);
            }
          }
          break;
        default:
          throw new Error(`Unsupported architecture on Linux: ${arch}`);
      }
      break;

    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  if (!nativeBinding) {
    throw new Error('Failed to load native binding');
  }
}

// Load the native binding
loadNativeBinding();

// Extract exports from native binding
const {
  ScreenCapture,
  getVersion,
  getSupportedPlatforms,
} = nativeBinding;

// Enhanced ScreenCapture class with convenience methods
class EnhancedScreenCapture extends ScreenCapture {
  constructor(options = {}) {
    super();
    this.isInitialized = false;
    this.isCapturing = false;
    this.frameCallback = null;
    this.errorCallback = null;
    
    if (options.autoInit !== false) {
      this.initialize(options);
    }
  }

  initialize(options = {}) {
    if (this.isInitialized) {
      throw new Error('ScreenCapture already initialized');
    }

    if (!ScreenCapture.isSupported()) {
      throw new Error('Screen capture is not supported on this platform');
    }

    if (!ScreenCapture.hasPermission()) {
      if (options.autoRequestPermission !== false) {
        const granted = ScreenCapture.requestPermission();
        if (!granted) {
          throw new Error('Screen capture permission denied');
        }
      } else {
        throw new Error('Screen capture permission required');
      }
    }

    // Set up default recording options with audio support
    const recordingOptions = {
      fps: 30,
      showCursor: true,
      outputType: 'BGRA',
      outputResolution: 'captured',
      captureSystemAudio: false,
      excludeCurrentProcessAudio: true,
      audioSampleRate: 48000,
      audioChannelCount: 2,
      captureMicrophone: false,
      ...options.recording
    };

    this.createCapturer(recordingOptions);
    this.isInitialized = true;
  }

  async startCaptureAsync(frameCallback, errorCallback) {
    if (!this.isInitialized) {
      throw new Error('ScreenCapture not initialized');
    }

    this.frameCallback = frameCallback;
    this.errorCallback = errorCallback;
    this.isCapturing = true;

    this.startCapture();

    // Start frame capture loop
    this._captureLoop();
  }

  _captureLoop() {
    if (!this.isCapturing) return;

    try {
      const frame = this.getNextFrame();
      if (this.frameCallback) {
        this.frameCallback(frame);
      }
    } catch (error) {
      if (this.errorCallback) {
        this.errorCallback(error);
      } else {
        console.error('Frame capture error:', error);
      }
    }

    // Continue capture loop
    if (this.isCapturing) {
      setImmediate(() => this._captureLoop());
    }
  }

  stopCaptureAsync() {
    this.isCapturing = false;
    this.stopCapture();
    this.frameCallback = null;
    this.errorCallback = null;
  }

  // Static convenience methods
  static async getAvailableTargets() {
    return ScreenCapture.getAllTargets();
  }

  static async checkPermissions() {
    return {
      supported: ScreenCapture.isSupported(),
      hasPermission: ScreenCapture.hasPermission(),
    };
  }

  static async requestPermissions() {
    if (!ScreenCapture.isSupported()) {
      throw new Error('Screen capture not supported');
    }
    return ScreenCapture.requestPermission();
  }
}

// Utility functions
function isSupported() {
  return ScreenCapture.isSupported();
}

function hasPermission() {
  return ScreenCapture.hasPermission();
}

function requestPermission() {
  return ScreenCapture.requestPermission();
}

function getAllTargets() {
  return ScreenCapture.getAllTargets();
}

// Main exports
module.exports = {
  ScreenCapture: EnhancedScreenCapture,
  isSupported,
  hasPermission,
  requestPermission,
  getAllTargets,
  getVersion,
  getSupportedPlatforms,
  
  // Legacy compatibility
  default: {
    ScreenCapture: EnhancedScreenCapture,
    isSupported,
    hasPermission,
    requestPermission,
    getAllTargets,
    getVersion,
    getSupportedPlatforms,
  }
};

// ES6 default export for modern usage
module.exports.default = module.exports;