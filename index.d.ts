/* TypeScript definitions for scap */

export interface ScreenSource {
  id: string;
  name: string;
  width: number;
  height: number;
  isDisplay: boolean;
}

export interface CropArea {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RecordingOptions {
  fps?: number;
  showCursor?: boolean;
  showHighlight?: boolean;
  outputType?: 'BGRA' | 'RGB' | 'BGR0' | 'YUV';
  outputResolution?: '480p' | '720p' | '1080p' | '1440p' | '2160p' | '4320p' | 'captured';
  cropArea?: CropArea;
  excludedTargets?: string[];
  captureSystemAudio?: boolean;
  excludeCurrentProcessAudio?: boolean;
  audioSampleRate?: number;
  audioChannelCount?: number;
  captureMicrophone?: boolean;
  microphoneDeviceId?: string;
}

export interface FrameData {
  width: number;
  height: number;
  data: Uint8Array;
  displayTime: number;
  frameType: string;
}

export interface AudioData {
  sampleRate: number;
  channelCount: number;
  sampleCount: number;
  data: Uint8Array;
  displayTime: number;
  bitsPerSample: number;
  source: 'system' | 'microphone';
}

export declare class ScreenCapture {
  constructor();
  
  static isSupported(): boolean;
  static hasPermission(): boolean;
  static requestPermission(): boolean;
  static getAllTargets(): ScreenSource[];
  
  createCapturer(options?: RecordingOptions): void;
  startCapture(): void;
  stopCapture(): void;
  getNextFrame(): FrameData;
  getOutputFrameSize(): [number, number];
}

export declare function getVersion(): string;
export declare function getSupportedPlatforms(): string[];

// Utility functions
export declare function isSupported(): boolean;
export declare function hasPermission(): boolean;
export declare function requestPermission(): boolean;
export declare function getAllTargets(): ScreenSource[];

// Default export for convenience
declare const scap: {
  ScreenCapture: typeof ScreenCapture;
  isSupported: typeof isSupported;
  hasPermission: typeof hasPermission;
  requestPermission: typeof requestPermission;
  getAllTargets: typeof getAllTargets;
  getVersion: typeof getVersion;
  getSupportedPlatforms: typeof getSupportedPlatforms;
};

export default scap;