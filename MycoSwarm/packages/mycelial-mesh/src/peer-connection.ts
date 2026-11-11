import type { MeshConfig } from './config.js';

export type SignalCallback = (data: unknown) => void;
export type ConnectCallback = () => void;
export type DataCallback = (data: Uint8Array) => void;
export type CloseCallback = () => void;

export class PeerConnectionWrapper {
  private pc: RTCPeerConnection;
  private dataChannel: RTCDataChannel | null = null;
  private _connected = false;

  onSignal: SignalCallback | null = null;
  onConnect: ConnectCallback | null = null;
  onData: DataCallback | null = null;
  onClose: CloseCallback | null = null;

  constructor(config: MeshConfig) {
    this.pc = new RTCPeerConnection({
      iceServers: [
        ...config.stunServers.map((url) => ({ urls: url })),
        ...config.turnServers,
      ],
    });
    this.setupListeners();
  }

  get connected(): boolean {
    return this._connected;
  }

  get connectionState(): string {
    return this.pc.connectionState;
  }

  private setupListeners(): void {
    this.pc.onicecandidate = (e) => {
      if (e.candidate) {
        this.onSignal?.({
          type: 'ice',
          candidate: {
            candidate: e.candidate.candidate,
            sdpMid: e.candidate.sdpMid,
            sdpMLineIndex: e.candidate.sdpMLineIndex,
            usernameFragment: e.candidate.usernameFragment,
          },
        });
      }
    };

    this.pc.ondatachannel = (e) => {
      this.setupDataChannel(e.channel);
    };

    this.pc.onconnectionstatechange = () => {
      if (
        this.pc.connectionState === 'disconnected' ||
        this.pc.connectionState === 'failed'
      ) {
        this._connected = false;
        this.onClose?.();
      } else if (this.pc.connectionState === 'connected') {
        this._connected = true;
      }
    };
  }

  private setupDataChannel(channel: RTCDataChannel): void {
    this.dataChannel = channel;
    channel.binaryType = 'arraybuffer';
    channel.onopen = () => {
      this._connected = true;
      this.onConnect?.();
    };
    channel.onclose = () => {
      this._connected = false;
      this.onClose?.();
    };
    channel.onmessage = (e) => {
      const data = e.data as ArrayBuffer;
      this.onData?.(new Uint8Array(data));
    };
  }

  async createOffer(initiator = true): Promise<void> {
    if (initiator) {
      this.dataChannel = this.pc.createDataChannel('fungal', {
        ordered: true,
      });
      this.setupDataChannel(this.dataChannel);
    }
    const offer = await this.pc.createOffer();
    await this.pc.setLocalDescription(offer);
    const sdp = this.pc.localDescription;
    if (sdp) {
      this.onSignal?.({
        type: 'offer',
        sdp: { type: sdp.type, sdp: sdp.sdp },
      });
    }
  }

  async handleSignal(
    data: { type: string; sdp?: RTCSessionDescriptionInit; candidate?: RTCIceCandidateInit },
  ): Promise<void> {
    if (data.type === 'offer') {
      await this.pc.setRemoteDescription(new RTCSessionDescription(data.sdp!));
      const answer = await this.pc.createAnswer();
      await this.pc.setLocalDescription(answer);
      const sdp = this.pc.localDescription;
      if (sdp) {
        this.onSignal?.({
          type: 'answer',
          sdp: { type: sdp.type, sdp: sdp.sdp },
        });
      }
    } else if (data.type === 'answer') {
      if (this.pc.signalingState !== 'stable') {
        await this.pc.setRemoteDescription(new RTCSessionDescription(data.sdp!));
      }
    } else if (data.type === 'ice') {
      try {
        await this.pc.addIceCandidate(new RTCIceCandidate(data.candidate!));
      } catch {
        // ignore invalid candidates
      }
    }
  }

  send(data: Uint8Array): void {
    if (this.dataChannel?.readyState === 'open') {
      this.dataChannel.send(data.buffer.slice(0) as ArrayBuffer);
    }
  }

  destroy(): void {
    this.dataChannel?.close();
    this.pc.close();
  }
}