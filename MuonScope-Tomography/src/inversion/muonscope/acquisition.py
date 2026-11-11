"""Event sources and the framing reader.

The processing chain must not know or care where bytes came from. Everything
downstream of :class:`PacketReader` sees decoded :class:`~muonscope.wire.EventRecord`
objects and nothing else, so the identical code path runs whether the bytes
arrive from a serial port, a recorded file, or the instrument simulator. That is
what makes a simulation result evidence about the software: it exercises the real
decoder, the real calibration, the real tracker and the real reconstructor.

Sources
-------
* :class:`MemorySource`     -- bytes already in hand (tests, replay of a capture)
* :class:`FileSource`       -- a recorded ``.msc`` capture, streamed in chunks
* :class:`SerialSource`     -- a live instrument over a serial port
* :class:`SimulatedSource`  -- the instrument simulator, emitting real wire bytes

The simulated source is deliberately *not* a shortcut: it runs the same physics
and the same detector readout, then serialises through
:func:`muonscope.wire.encode_packet` and hands the result to the same reader. A
bug in the encoder is a bug in the simulation, which is the point.
"""
from __future__ import annotations

import abc
import os
import time
from dataclasses import dataclass
from typing import Iterable, Iterator, List, Optional, Sequence, Tuple

import numpy as np

from .wire import (
    MAGIC,
    MAX_HITS,
    MIN_PACKET_SIZE,
    N_HITS_OFFSET,
    EventRecord,
    ProtocolError,
    decode_packet,
    encode_packet,
    packet_length,
)

_MAGIC_BYTES = MAGIC.to_bytes(4, "big")


class EventSource(abc.ABC):
    """A byte stream carrying instrument packets."""

    name: str = "unnamed"
    is_live: bool = False

    def open(self) -> None:  # pragma: no cover - trivial
        pass

    def close(self) -> None:  # pragma: no cover - trivial
        pass

    @abc.abstractmethod
    def read(self, max_bytes: int = 65536) -> bytes:
        """Return up to ``max_bytes``; empty bytes means end of stream."""

    def __enter__(self) -> "EventSource":
        self.open()
        return self

    def __exit__(self, *exc) -> None:
        self.close()


class MemorySource(EventSource):
    """Bytes held in memory. Used by tests and by capture replay."""

    def __init__(self, data: bytes, name: str = "memory", chunk_size: int = 4096):
        self._data = bytes(data)
        self._pos = 0
        self._chunk = int(chunk_size)
        self.name = name

    def read(self, max_bytes: int = 65536) -> bytes:
        if self._pos >= len(self._data):
            return b""
        n = min(max_bytes, self._chunk, len(self._data) - self._pos)
        out = self._data[self._pos:self._pos + n]
        self._pos += n
        return out

    def __len__(self) -> int:
        return len(self._data)


class FileSource(EventSource):
    """Replay a recorded capture, optionally in real time."""

    def __init__(self, path: str, chunk_size: int = 65536,
                 realtime: bool = False, bitrate_bytes_per_s: float = 1e6):
        self.path = path
        self.name = os.path.basename(path)
        self._chunk = int(chunk_size)
        self._realtime = realtime
        self._bitrate = float(bitrate_bytes_per_s)
        self._handle = None

    def open(self) -> None:
        self._handle = open(self.path, "rb")

    def close(self) -> None:
        if self._handle is not None:
            self._handle.close()
            self._handle = None

    def read(self, max_bytes: int = 65536) -> bytes:
        if self._handle is None:
            self.open()
        chunk = self._handle.read(min(max_bytes, self._chunk))
        if chunk and self._realtime and self._bitrate > 0:
            time.sleep(len(chunk) / self._bitrate)
        return chunk


class SerialSource(EventSource):
    """A live instrument on a serial port.

    Requires ``pyserial``. The import is deferred and the failure is explicit:
    silently substituting a simulator for missing hardware is exactly the class of
    mistake this package exists to prevent.
    """

    is_live = True

    def __init__(self, port: str, baudrate: int = 3_000_000, timeout_s: float = 1.0):
        self.port = port
        self.baudrate = int(baudrate)
        self.timeout_s = float(timeout_s)
        self.name = f"serial:{port}"
        self._serial = None

    def open(self) -> None:
        try:
            import serial  # type: ignore
        except ImportError as exc:  # pragma: no cover - depends on environment
            raise RuntimeError(
                "pyserial is required for a live instrument: pip install pyserial"
            ) from exc
        self._serial = serial.Serial(self.port, self.baudrate, timeout=self.timeout_s)

    def close(self) -> None:
        if self._serial is not None:
            self._serial.close()
            self._serial = None

    def read(self, max_bytes: int = 65536) -> bytes:
        if self._serial is None:
            self.open()
        return self._serial.read(max_bytes)


class PacketReader:
    """Incremental framer: bytes in, decoded records out.

    Keeps a buffer across reads so a packet split over two chunks is not lost,
    which a real UART link will do routinely. Resynchronises on the magic word
    when a frame fails to decode, so one corrupt packet does not destroy the run.
    """

    def __init__(self, max_buffer_bytes: int = 8 * 1024 * 1024):
        self.buffer = bytearray()
        self.max_buffer_bytes = int(max_buffer_bytes)
        self.stats = {
            "bytes_in": 0,
            "packets_ok": 0,
            "packets_bad": 0,
            "bytes_discarded": 0,
            "last_error": None,
        }

    def feed(self, chunk: bytes) -> List[EventRecord]:
        """Append bytes and return every complete, valid record now available."""
        if chunk:
            self.buffer.extend(chunk)
            self.stats["bytes_in"] += len(chunk)
        out: List[EventRecord] = []
        i = 0
        buf = self.buffer
        n = len(buf)

        while i + MIN_PACKET_SIZE <= n:
            if buf[i:i + 4] != _MAGIC_BYTES:
                i += 1
                continue
            n_hits = int.from_bytes(buf[i + N_HITS_OFFSET:i + N_HITS_OFFSET + 2], "big")
            if n_hits > MAX_HITS:
                self.stats["packets_bad"] += 1
                i += 1
                continue
            need = packet_length(n_hits)
            if i + need > n:
                break
            try:
                record = decode_packet(bytes(buf[i:i + need]))
            except ProtocolError as exc:
                self.stats["packets_bad"] += 1
                self.stats["last_error"] = str(exc)
                i += 1
                continue
            self.stats["packets_ok"] += 1
            out.append(record)
            i += need

        if i:
            self.stats["bytes_discarded"] += i
            del buf[:i]
        if len(buf) > self.max_buffer_bytes:
            self.stats["bytes_discarded"] += len(buf)
            buf.clear()
        return out

    def records_from_source(
        self, source: EventSource, max_chunks: Optional[int] = None
    ) -> Iterator[EventRecord]:
        """Pull from a source until it is exhausted."""
        chunks = 0
        with source:
            while True:
                if max_chunks is not None and chunks >= max_chunks:
                    break
                chunk = source.read()
                if not chunk:
                    break
                chunks += 1
                for record in self.feed(chunk):
                    yield record


class CaptureWriter:
    """Append raw packets to a capture file.

    Written before any interpretation, so a run can be replayed exactly and a
    disputed result can be re-derived rather than re-argued.
    """

    def __init__(self, path: str):
        self.path = path
        self._handle = None
        self.bytes_written = 0

    def open(self) -> None:
        directory = os.path.dirname(os.path.abspath(self.path))
        if directory:
            os.makedirs(directory, exist_ok=True)
        self._handle = open(self.path, "wb")

    def write(self, record: EventRecord) -> int:
        if self._handle is None:
            self.open()
        payload = encode_packet(record)
        self._handle.write(payload)
        self.bytes_written += len(payload)
        return len(payload)

    def close(self) -> None:
        if self._handle is not None:
            self._handle.flush()
            self._handle.close()
            self._handle = None

    def __enter__(self) -> "CaptureWriter":
        self.open()
        return self

    def __exit__(self, *exc) -> None:
        self.close()


@dataclass
class SimulatorConfig:
    """Knobs for the instrument simulator."""

    num_events: int = 5000
    seed: int = 12345
    station_id: int = 1
    simulated_flag: bool = True
    include_dark_counts: bool = True
    dark_count_rate_hz_per_channel: float = 80_000.0
    clock_period_ns: float = 5.0        # 200 MHz system clock
    tdc_fine_bin_ps: float = 75.0
    start_timestamp_ns: int = 1_700_000_000_000_000_000
    tdc_start_latency_ns: float = 100.0
    # Fixed per-channel electronic delay, injected so the calibration has a real
    # skew to recover. A simulator that emitted perfectly aligned channels would
    # make a broken deskew routine look correct.
    channel_delay_ps_sigma: float = 250.0
    channel_delay_ps_uniform: float = 800.0
    # Timing jitter of the discriminator + TDC, on top of the fixed delays.
    time_jitter_ps: float = 80.0
    # Path length from the entry plane to a layer is (z_entry - z_layer)/|cos|;
    # light propagation in the strip is folded into this single term.
    propagation_speed_mm_per_ns: float = 299.792458
    # A calibration run: an internal pulser fanning out to every channel, with
    # the arrival phase scanned across the clock period. Firing all channels in
    # the same packet is what makes the deskew work: they share one phase, so the
    # per-packet mean removes the phase exactly and each channel's offset is
    # limited only by jitter, not by how well its phase happened to be sampled.
    # Physics data cannot do this -- its arrival times are clustered.
    calibration_packets: int = 400
    # Charge integration: full-scale ADC for a strip that captured all the light,
    # with read noise. The tracker's barycentre is only unbiased if ADC is
    # proportional to charge, so no pedestal is added here.
    adc_full_scale: float = 4000.0
    adc_noise_counts: float = 25.0


class SimulatedSource(EventSource):
    """The instrument simulator.

    Runs the forward physics and the detector readout, then emits the result as
    wire packets. Because the output is bytes, the software under test cannot
    tell this apart from hardware except by the ``FLAG_SIMULATED`` bit.
    """

    def __init__(
        self,
        scene,
        geometry,
        detector_response=None,
        config: Optional[SimulatorConfig] = None,
        tracer=None,
    ):
        from .physics.gaisser import sample_cosmic_muons
        from .simulation.detector import DetectorResponse, HodoscopeSimulator
        from .simulation.ray_tracer import MonteCarloRayTracer

        self.scene = scene
        self.geometry = geometry
        self.config = config or SimulatorConfig()
        self.response = detector_response or DetectorResponse()
        self.rng = np.random.default_rng(self.config.seed)
        self._sample_muons = sample_cosmic_muons

        self._tracer = tracer or MonteCarloRayTracer(
            scene,
            top_panel_z_mm=geometry.entry_exit_z_mm()[0],
            bottom_panel_z_mm=geometry.entry_exit_z_mm()[1],
        )
        self._hodoscope = HodoscopeSimulator(
            geometry, self.response, rng=np.random.default_rng(self.config.seed + 1))
        self._pending: List[bytes] = []
        self._sequence = 0
        self.name = "simulator"
        self.generated_events = 0
        self.detected_events = 0
        # Per-channel fixed electronic delay, drawn once. The calibration is
        # supposed to find this; that is what makes the deskew test meaningful.
        n_channels = self._channel_map().num_channels()
        delay_rng = np.random.default_rng(self.config.seed + 4)
        self.channel_delay_ps = (
            delay_rng.normal(0.0, self.config.channel_delay_ps_sigma, size=n_channels)
            + delay_rng.uniform(-0.5, 0.5, size=n_channels)
            * self.config.channel_delay_ps_uniform
        )
        # Side channel for validation only. It is deliberately *not* part of the
        # source protocol: MuonScopeSystem cannot reach it, so the truth cannot
        # leak into the reconstruction. Use compare_to_truth() afterwards.
        self.truth_by_sequence: Dict[int, Tuple[float, float]] = {}

    def _encode_time(self, time_ns: float) -> Tuple[int, int]:
        """Quantise an absolute hit time onto the coarse counter plus TDC code.

        The coarse counter ticks at the system clock; the fine code subdivides
        one tick into ``clock_period_ns / fine_bin`` bins. A code that rounds up
        to a full tick carries into the coarse counter, which is what real TDC
        logic does and what a naive encoder gets wrong.
        """
        cfg = self.config
        clock_ns = cfg.clock_period_ns
        bins_per_tick = (clock_ns * 1000.0) / cfg.tdc_fine_bin_ps
        coarse = int(np.floor(time_ns / clock_ns))
        frac_ns = time_ns - coarse * clock_ns
        fine = int(round(frac_ns * 1000.0 / cfg.tdc_fine_bin_ps))
        if fine >= bins_per_tick:
            fine = 0
            coarse += 1
        return coarse & 0xFFFF, fine & 0xFFFF

    # -- generation -----------------------------------------------------------

    def _channel_map(self):
        from .tracking import ChannelMap
        return ChannelMap(self.geometry)

    def generate(self) -> List[bytes]:
        """Produce the whole run as encoded packets: calibration, then physics."""
        cfg = self.config
        cmap = self._channel_map()
        packets: List[bytes] = []

        if cfg.calibration_packets > 0:
            packets.extend(self._calibration_packets(cmap))

        energies, directions, charges = self._sample_muons(
            cfg.num_events, rng=np.random.default_rng(cfg.seed + 2))
        truth = self._tracer.trace_batch(
            cfg.num_events, energies, directions, charges,
            rng=np.random.default_rng(cfg.seed + 3))
        self.generated_events = len(truth)

        base_ts = cfg.start_timestamp_ns
        seq = len(packets)
        z_entry = self.geometry.entry_exit_z_mm()[0]

        for event in truth:
            measured = self._hodoscope.measure(event)
            if measured is None:
                continue
            self.detected_events += 1

            hits = []
            trigger_mask = 0
            cz = float(measured.d_in_measured[2])
            cos_in = abs(cz) if abs(cz) > 1e-6 else 1e-6
            for layer in measured.layers:
                plane = cmap.plane_for(layer.plane_id)
                if plane is None:
                    continue
                global_ch = cmap.global_channel(plane, layer.strip_index)
                # Time of flight from the entry plane to this layer, plus the
                # fixed per-channel electronic delay, plus jitter.
                path_mm = abs(z_entry - layer.z_mm) / cos_in
                tof_ns = path_mm / cfg.propagation_speed_mm_per_ns
                delay_ns = float(self.channel_delay_ps[global_ch]) / 1000.0
                jitter_ns = float(self.rng.normal(0.0, cfg.time_jitter_ps)) / 1000.0
                t_ns = cfg.tdc_start_latency_ns + tof_ns + delay_ns + jitter_ns
                coarse, fine = self._encode_time(t_ns)
                adc_raw = (layer.charge_fraction * cfg.adc_full_scale
                           + self.rng.normal(0.0, cfg.adc_noise_counts))
                adc = int(np.clip(round(adc_raw), 0, 65535))
                from .wire import HitRecord
                hits.append(HitRecord(channel=global_ch, tdc_coarse=coarse,
                                      tdc_fine=fine, adc=adc, flags=0))
                trigger_mask |= 1 << cmap.plane_index(layer.plane_id)

            if cfg.include_dark_counts:
                hits.extend(self._dark_counts(cmap))

            flags = 0x01 if cfg.simulated_flag else 0x00
            from .wire import EventRecord
            record = EventRecord(
                sequence=seq,
                timestamp_utc_ns=base_ts + seq * 40_000_000,   # ~25 Hz trigger rate
                station_id=cfg.station_id,
                trigger_mask=trigger_mask,
                hits=hits,
                flags=flags,
            )
            packets.append(encode_packet(record))
            self.truth_by_sequence[seq] = (
                float(event.scattering_angle_rad), float(event.energy_initial_gev))
            seq += 1

        self._sequence = seq
        return packets

    def _calibration_packets(self, cmap) -> List[bytes]:
        """Emit a pulser-scan calibration run.

        Every channel fires in every packet with a shared arrival phase, which is
        stepped across the clock period so the fine codes tile their range. The
        injected per-channel electronic delay is present, and is what the deskew
        step has to recover.
        """
        from .wire import FLAG_CALIBRATION, EventRecord, HitRecord
        cfg = self.config
        clock_ps = cfg.clock_period_ns * 1000.0
        n_channels = cmap.num_channels()
        packets: List[bytes] = []
        rng = np.random.default_rng(cfg.seed + 5)

        for seq in range(cfg.calibration_packets):
            phase_ps = float(rng.uniform(0.0, clock_ps))
            shared_jitter_ps = float(rng.normal(0.0, cfg.time_jitter_ps))
            hits = []
            for ch in range(n_channels):
                delay_ps = float(self.channel_delay_ps[ch])
                # Per-channel readout jitter on top of the shared pulse.
                jitter_ps = shared_jitter_ps + float(rng.normal(0.0, cfg.time_jitter_ps))
                t_ns = cfg.tdc_start_latency_ns + (phase_ps + delay_ps + jitter_ps) / 1000.0
                coarse, fine = self._encode_time(t_ns)
                hits.append(HitRecord(channel=ch, tdc_coarse=coarse, tdc_fine=fine,
                                      adc=1500, flags=0))
            packets.append(encode_packet(EventRecord(
                sequence=seq,
                timestamp_utc_ns=cfg.start_timestamp_ns + seq * 1_000_000,
                station_id=cfg.station_id,
                trigger_mask=0xFFFF,
                hits=hits,
                flags=FLAG_CALIBRATION | (0x01 if cfg.simulated_flag else 0x00),
            )))
        return packets

    def _dark_counts(self, cmap) -> List:
        """Uncorrelated SiPM dark counts inside the readout window."""
        from .wire import HIT_FLAG_DARK_COUNT, HitRecord
        window_s = 200e-9
        expected = self.config.dark_count_rate_hz_per_channel * window_s * cmap.num_channels()
        n_dark = int(self.rng.poisson(expected))
        out = []
        for _ in range(n_dark):
            global_ch = int(self.rng.integers(0, cmap.num_channels()))
            coarse, fine = self._encode_time(
                self.config.tdc_start_latency_ns
                + float(self.rng.uniform(0.0, 40.0)))
            out.append(HitRecord(
                channel=global_ch,
                tdc_coarse=coarse,
                tdc_fine=fine,
                adc=int(self.rng.integers(100, 300)),
                flags=HIT_FLAG_DARK_COUNT,
            ))
        return out

    # -- EventSource ----------------------------------------------------------

    def open(self) -> None:
        if not self._pending:
            self._pending = self.generate()
        self._cursor = 0

    def read(self, max_bytes: int = 65536) -> bytes:
        if not hasattr(self, "_cursor"):
            self.open()
        out = bytearray()
        while self._cursor < len(self._pending) and len(out) < max_bytes:
            out.extend(self._pending[self._cursor])
            self._cursor += 1
        return bytes(out)
