// examples/electron/main.js - Electron main process example

const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { ScreenCapture } = require('scap');

let mainWindow;
let screenCapture;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  mainWindow.loadFile('index.html');
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (screenCapture) {
    try {
      screenCapture.stopCaptureAsync();
    } catch (error) {
      console.error('Error stopping capture:', error);
    }
  }
  
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

// IPC handlers for screen capture
ipcMain.handle('screen-capture:check-support', async () => {
  try {
    return await ScreenCapture.checkPermissions();
  } catch (error) {
    return { error: error.message };
  }
});

ipcMain.handle('screen-capture:request-permission', async () => {
  try {
    return await ScreenCapture.requestPermissions();
  } catch (error) {
    throw error;
  }
});

ipcMain.handle('screen-capture:get-targets', async () => {
  try {
    return await ScreenCapture.getAvailableTargets();
  } catch (error) {
    throw error;
  }
});

ipcMain.handle('screen-capture:start', async (event, options = {}) => {
  try {
    screenCapture = new ScreenCapture({
      autoInit: true,
      recording: {
        fps: options.fps || 30,
        showCursor: options.showCursor !== false,
        outputType: options.outputType || 'BGRA',
        outputResolution: options.outputResolution || '720p',
        cropArea: options.cropArea,
        captureSystemAudio: options.captureSystemAudio || false,
        excludeCurrentProcessAudio: options.excludeCurrentProcessAudio !== false,
        audioSampleRate: options.audioSampleRate || 48000,
        audioChannelCount: options.audioChannelCount || 2,
        captureMicrophone: options.captureMicrophone || false,
        microphoneDeviceId: options.microphoneDeviceId
      }
    });

    // Start capture with frame callback
    await screenCapture.startCaptureAsync(
      (frame) => {
        // Send frame data to renderer process
        mainWindow.webContents.send('screen-capture:frame', {
          width: frame.width,
          height: frame.height,
          data: Array.from(frame.data), // Convert Uint8Array to regular array for IPC
          displayTime: frame.displayTime,
          frameType: frame.frameType
        });
      },
      (error) => {
        console.error('Screen capture error:', error);
        mainWindow.webContents.send('screen-capture:error', error.message);
      }
    );

    return { success: true };
  } catch (error) {
    throw error;
  }
});

ipcMain.handle('screen-capture:stop', async () => {
  try {
    if (screenCapture) {
      await screenCapture.stopCaptureAsync();
      screenCapture = null;
    }
    return { success: true };
  } catch (error) {
    throw error;
  }
});

ipcMain.handle('screen-capture:get-frame-size', async () => {
  try {
    if (screenCapture) {
      const [width, height] = screenCapture.getOutputFrameSize();
      return { width, height };
    }
    return null;
  } catch (error) {
    throw error;
  }
});

// Preload script (preload.js)
const preloadScript = `
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('screenCapture', {
  checkSupport: () => ipcRenderer.invoke('screen-capture:check-support'),
  requestPermission: () => ipcRenderer.invoke('screen-capture:request-permission'),
  getTargets: () => ipcRenderer.invoke('screen-capture:get-targets'),
  start: (options) => ipcRenderer.invoke('screen-capture:start', options),
  stop: () => ipcRenderer.invoke('screen-capture:stop'),
  getFrameSize: () => ipcRenderer.invoke('screen-capture:get-frame-size'),
  
  // Event listeners
  onFrame: (callback) => {
    ipcRenderer.on('screen-capture:frame', (event, frame) => callback(frame));
  },
  onError: (callback) => {
    ipcRenderer.on('screen-capture:error', (event, error) => callback(error));
  },
  
  // Remove listeners
  removeAllListeners: () => {
    ipcRenderer.removeAllListeners('screen-capture:frame');
    ipcRenderer.removeAllListeners('screen-capture:error');
  }
});
`;

// Renderer HTML (index.html)
const rendererHTML = `
<!DOCTYPE html>
<html>
<head>
  <title>Screen Capture Demo</title>
  <style>
    body {
      font-family: Arial, sans-serif;
      margin: 20px;
      background: #f0f0f0;
    }
    .container {
      max-width: 1000px;
      margin: 0 auto;
      background: white;
      padding: 20px;
      border-radius: 8px;
      box-shadow: 0 2px 10px rgba(0,0,0,0.1);
    }
    .controls {
      margin-bottom: 20px;
    }
    button {
      padding: 10px 20px;
      margin: 5px;
      border: none;
      border-radius: 4px;
      background: #007acc;
      color: white;
      cursor: pointer;
    }
    button:hover {
      background: #005a9e;
    }
    button:disabled {
      background: #ccc;
      cursor: not-allowed;
    }
    .preview {
      border: 1px solid #ddd;
      border-radius: 4px;
      margin-top: 20px;
    }
    canvas {
      display: block;
      max-width: 100%;
      height: auto;
    }
    .info {
      margin-top: 10px;
      padding: 10px;
      background: #f8f9fa;
      border-radius: 4px;
      font-family: monospace;
      font-size: 12px;
    }
    .error {
      color: #dc3545;
      background: #f8d7da;
      border: 1px solid #f5c6cb;
    }
    .success {
      color: #155724;
      background: #d4edda;
      border: 1px solid #c3e6cb;
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>Screen Capture Demo</h1>
    
    <div class="controls">
      <button id="check-support">Check Support</button>
      <button id="request-permission">Request Permission</button>
      <button id="get-targets">Get Targets</button>
      <button id="start-capture">Start Capture</button>
      <button id="stop-capture">Stop Capture</button>
    </div>
    
    <div id="status" class="info">Ready</div>
    
    <div class="preview">
      <canvas id="preview-canvas" width="640" height="480"></canvas>
    </div>
    
    <div id="frame-info" class="info">
      Frame info will appear here
    </div>
  </div>

  <script>
    const canvas = document.getElementById('preview-canvas');
    const ctx = canvas.getContext('2d');
    const statusEl = document.getElementById('status');
    const frameInfoEl = document.getElementById('frame-info');
    
    let isCapturing = false;
    let frameCount = 0;
    let startTime = Date.now();
    
    // Utility functions
    function updateStatus(message, type = 'info') {
      statusEl.textContent = message;
      statusEl.className = \`info \${type}\`;
    }
    
    function updateFrameInfo(frame) {
      frameCount++;
      const elapsed = (Date.now() - startTime) / 1000;
      const fps = frameCount / elapsed;
      
      frameInfoEl.innerHTML = \`
        Frame #\${frameCount} | 
        Size: \${frame.width}x\${frame.height} | 
        Type: \${frame.frameType} | 
        FPS: \${fps.toFixed(1)} | 
        Data: \${frame.data.length} bytes
      \`;
    }
    
    // Event handlers
    document.getElementById('check-support').addEventListener('click', async () => {
      try {
        const support = await window.screenCapture.checkSupport();
        updateStatus(\`Support: \${JSON.stringify(support)}\`, 'success');
      } catch (error) {
        updateStatus(\`Error: \${error.message}\`, 'error');
      }
    });
    
    document.getElementById('request-permission').addEventListener('click', async () => {
      try {
        const granted = await window.screenCapture.requestPermission();
        updateStatus(\`Permission granted: \${granted}\`, granted ? 'success' : 'error');
      } catch (error) {
        updateStatus(\`Error: \${error.message}\`, 'error');
      }
    });
    
    document.getElementById('get-targets').addEventListener('click', async () => {
      try {
        const targets = await window.screenCapture.getTargets();
        updateStatus(\`Found \${targets.length} targets\`, 'success');
        console.log('Available targets:', targets);
      } catch (error) {
        updateStatus(\`Error: \${error.message}\`, 'error');
      }
    });
    
    document.getElementById('start-capture').addEventListener('click', async () => {
      if (isCapturing) return;
      
      try {
        frameCount = 0;
        startTime = Date.now();
        
        await window.screenCapture.start({
          fps: 30,
          showCursor: true,
          outputType: 'BGRA',
          outputResolution: '720p'
        });
        
        isCapturing = true;
        updateStatus('Screen capture started', 'success');
        
        // Get frame size and resize canvas
        const frameSize = await window.screenCapture.getFrameSize();
        if (frameSize) {
          const scale = Math.min(640 / frameSize.width, 480 / frameSize.height);
          canvas.width = frameSize.width * scale;
          canvas.height = frameSize.height * scale;
        }
        
      } catch (error) {
        updateStatus(\`Error: \${error.message}\`, 'error');
      }
    });
    
    document.getElementById('stop-capture').addEventListener('click', async () => {
      if (!isCapturing) return;
      
      try {
        await window.screenCapture.stop();
        isCapturing = false;
        updateStatus('Screen capture stopped', 'success');
      } catch (error) {
        updateStatus(\`Error: \${error.message}\`, 'error');
      }
    });
    
    // Frame handler
    window.screenCapture.onFrame((frame) => {
      if (!isCapturing) return;
      
      updateFrameInfo(frame);
      
      // Convert frame data to ImageData and draw to canvas
      if (frame.frameType === 'BGRA' && frame.data.length > 0) {
        const imageData = ctx.createImageData(frame.width, frame.height);
        
        // Convert BGRA to RGBA for canvas
        for (let i = 0; i < frame.data.length; i += 4) {
          imageData.data[i] = frame.data[i + 2];     // R
          imageData.data[i + 1] = frame.data[i + 1]; // G
          imageData.data[i + 2] = frame.data[i];     // B
          imageData.data[i + 3] = frame.data[i + 3]; // A
        }
        
        // Create temporary canvas for scaling
        const tempCanvas = document.createElement('canvas');
        const tempCtx = tempCanvas.getContext('2d');
        tempCanvas.width = frame.width;
        tempCanvas.height = frame.height;
        tempCtx.putImageData(imageData, 0, 0);
        
        // Draw scaled to main canvas
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(tempCanvas, 0, 0, canvas.width, canvas.height);
      }
    });
    
    // Error handler
    window.screenCapture.onError((error) => {
      updateStatus(\`Capture error: \${error}\`, 'error');
      isCapturing = false;
    });
    
    // Cleanup on page unload
    window.addEventListener('beforeunload', () => {
      window.screenCapture.removeAllListeners();
    });
  </script>
</body>
</html>
`;

// Write files if this script is run directly
if (require.main === module) {
  const fs = require('fs');
  const path = require('path');
  
  const exampleDir = path.join(__dirname, 'examples', 'electron');
  
  // Create directory if it doesn't exist
  if (!fs.existsSync(exampleDir)) {
    fs.mkdirSync(exampleDir, { recursive: true });
  }
  
  // Write preload script
  fs.writeFileSync(path.join(exampleDir, 'preload.js'), preloadScript);
  
  // Write HTML file
  fs.writeFileSync(path.join(exampleDir, 'index.html'), rendererHTML);
  
  console.log('Example files created in examples/electron/');
}