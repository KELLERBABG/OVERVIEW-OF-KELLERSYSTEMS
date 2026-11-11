"""MuonScope binary wire protocol.

This is the interface between the instrument and the software. It is the only
thing the processing chain is allowed to consume: a reconstructor that has access
to anything else (the simulator's true energy, the true scattering angle) is not
being tested.

The format
----------
All integers are big-endian. A packet is::

    offset  size  field
    ------  ----  ---------------------------------------------------------
         0     4  magic            0xAA55AA55
         4     1  version          protocol version (currently 1)
         5     1  flags            bit 0: simulated, bit 1: calibration run
         6     4  sequence         monotonic event counter, wraps at 2^32
        10     8  timestamp_utc_ns nanoseconds since the Unix epoch (GNSS PPS)
        18     1  station_id       0-255
        19     2  trigger_mask       bit per plane that met the coincidence
        21     2  n_hits             number of hit records that follow
        23  9*n  hits               see below
     23+9n     4  crc32            CRC-32/ISO-HDLC over bytes [0, 23+9n)

Each hit record is::

    size  field
    ----  --------------------------------------------------------------
       2  channel        global strip channel, 0..511 (see geometry bank map)
       2  tdc_coarse     clock ticks since the trigger
       2  tdc_fine       tapped-delay-line code within one clock
       2  adc            charge-integrated pulse height, ADC counts
       1  hit_flags      bit 0: dark-count flagged, bit 1: channel masked

Packet length is ``27 + 9 * n_hits``.

What this replaces
------------------
The repository previously documented a "fixed-size 32-byte packet" whose own
header fields already consumed all 32 bytes, so a four-plane event could not fit
in it, and the decoder's minimum-length gate admitted buffers too short to
contain a CRC. Both are fixed here, and the number of hits is authoritative
rather than implied, so the frame is self-describing.

Nothing in the packet carries momentum, energy, or scattering angle. Those are
not measurable by the instrument, and the point of this format is that the
software cannot accidentally use them.
"""
from __future__ import annotations

import struct
from dataclasses import dataclass, field
from typing import Iterable, Iterator, List, Optional, Sequence, Tuple

MAGIC = 0xAA55AA55

VERSION = 1

FLAG_SIMULATED = 0x01
FLAG_CALIBRATION = 0x02

# Hit flags
HIT_FLAG_DARK_COUNT = 0x01
HIT_FLAG_MASKED = 0x02

# magic u32, version u8, flags u8, sequence u32, timestamp u64, station u8,
# trigger_mask u16, n_hits u16  ->  4+1+1+4+8+1+2+2 = 23 bytes
_HEADER = struct.Struct(">IBBIQBHH")
_HIT = struct.Struct(">HHHHB")
_CRC = struct.Struct(">I")

HEADER_SIZE = _HEADER.size          # 23
HIT_SIZE = _HIT.size                # 9
CRC_SIZE = _CRC.size                # 4
MIN_PACKET_SIZE = HEADER_SIZE + CRC_SIZE   # 27, for a zero-hit packet
MAX_HITS = 4096

# Byte offset of the hit-count field inside the header.
N_HITS_OFFSET = 21


class ProtocolError(ValueError):
    """Raised when a buffer is not a valid MuonScope packet."""


@dataclass
class HitRecord:
    channel: int
    tdc_coarse: int
    tdc_fine: int
    adc: int
    flags: int = 0

    @property
    def is_dark_count(self) -> bool:
        return bool(self.flags & HIT_FLAG_DARK_COUNT)

    @property
    def is_masked(self) -> bool:
        return bool(self.flags & HIT_FLAG_MASKED)


@dataclass
class EventRecord:
    """One decoded instrument trigger. Contains only measurable quantities."""

    sequence: int
    timestamp_utc_ns: int
    station_id: int
    trigger_mask: int
    hits: List[HitRecord] = field(default_factory=list)
    version: int = VERSION
    flags: int = 0

    @property
    def n_hits(self) -> int:
        return len(self.hits)

    @property
    def simulated(self) -> bool:
        return bool(self.flags & FLAG_SIMULATED)

    def live_hits(self) -> List[HitRecord]:
        """Hits that survive masking and dark-count rejection."""
        return [h for h in self.hits if not h.is_masked and not h.is_dark_count]


def encode_packet(record: EventRecord) -> bytes:
    """Serialise an :class:`EventRecord` to bytes, CRC included."""
    n = len(record.hits)
    if n > MAX_HITS:
        raise ProtocolError(f"{n} hits exceeds MAX_HITS={MAX_HITS}")
    if not 0 <= record.station_id <= 0xFF:
        raise ProtocolError(f"station_id {record.station_id} out of range")

    header = _HEADER.pack(
        MAGIC,
        record.version & 0xFF,
        record.flags & 0xFF,
        record.sequence & 0xFFFFFFFF,
        record.timestamp_utc_ns & 0xFFFFFFFFFFFFFFFF,
        record.station_id & 0xFF,
        record.trigger_mask & 0xFFFF,
        n & 0xFFFF,
    )
    body = bytearray(header)
    for hit in record.hits:
        body += _HIT.pack(
            hit.channel & 0xFFFF,
            hit.tdc_coarse & 0xFFFF,
            hit.tdc_fine & 0xFFFF,
            hit.adc & 0xFFFF,
            hit.flags & 0xFF,
        )
    return bytes(body) + _CRC.pack(crc32(bytes(body)))


def packet_length(n_hits: int) -> int:
    return HEADER_SIZE + HIT_SIZE * int(n_hits) + CRC_SIZE


def decode_packet(buffer: bytes, verify_crc: bool = True) -> EventRecord:
    """Parse exactly one packet.

    Raises :class:`ProtocolError` on a short buffer, a bad magic word, an
    impossible hit count, a length mismatch, or a CRC failure. A real acquisition
    path must be able to reject bad frames loudly rather than mis-parse them.
    """
    if len(buffer) < MIN_PACKET_SIZE:
        raise ProtocolError(
            f"buffer of {len(buffer)} bytes is shorter than the {MIN_PACKET_SIZE}-byte minimum")

    magic, version, flags, sequence, timestamp, station, trigger, n_hits = _HEADER.unpack_from(buffer, 0)
    if magic != MAGIC:
        raise ProtocolError(f"bad magic 0x{magic:08X}, expected 0x{MAGIC:08X}")
    if n_hits > MAX_HITS:
        raise ProtocolError(f"hit count {n_hits} exceeds MAX_HITS={MAX_HITS}")

    expected = packet_length(n_hits)
    if len(buffer) < expected:
        raise ProtocolError(
            f"declared {n_hits} hits need {expected} bytes, only {len(buffer)} present")

    hits: List[HitRecord] = []
    offset = HEADER_SIZE
    for _ in range(n_hits):
        channel, coarse, fine, adc, hflags = _HIT.unpack_from(buffer, offset)
        hits.append(HitRecord(channel=channel, tdc_coarse=coarse, tdc_fine=fine,
                              adc=adc, flags=hflags))
        offset += HIT_SIZE

    (stored_crc,) = _CRC.unpack_from(buffer, offset)
    if verify_crc:
        computed = crc32(buffer[:offset])
        if computed != stored_crc:
            raise ProtocolError(
                f"CRC mismatch: stored 0x{stored_crc:08X}, computed 0x{computed:08X}")

    return EventRecord(
        sequence=sequence,
        timestamp_utc_ns=timestamp,
        station_id=station,
        trigger_mask=trigger,
        hits=hits,
        version=version,
        flags=flags,
    )


def crc32(data: bytes) -> int:
    """CRC-32/ISO-HDLC, the CRC-32 named in the interface specification."""
    import zlib
    return zlib.crc32(data) & 0xFFFFFFFF


def iter_packets(
    stream: bytes,
    verify_crc: bool = True,
    on_error: Optional[str] = None,
) -> Iterator[Tuple[int, EventRecord]]:
    """Scan a byte stream for packets, resynchronising on the magic word.

    Yields ``(byte_offset, record)``. A corrupt frame is skipped by advancing one
    byte past the false magic and rescanning, which is what a UART link needs:
    losing byte alignment must not lose the rest of the run.
    """
    i = 0
    n = len(stream)
    while i + MIN_PACKET_SIZE <= n:
        if stream[i:i + 4] != _MAGIC_BYTES:
            i += 1
            continue
        try:
            n_hits = _HEADER.unpack_from(stream, i)[7]
        except struct.error:
            break
        if n_hits > MAX_HITS:
            i += 1
            continue
        need = packet_length(n_hits)
        if i + need > n:
            break  # truncated tail: wait for more bytes
        try:
            record = decode_packet(stream[i:i + need], verify_crc=verify_crc)
        except ProtocolError as exc:
            if on_error is not None:
                on_error = str(exc)
            i += 1
            continue
        yield i, record
        i += need


_MAGIC_BYTES = struct.pack(">I", MAGIC)


def describe_layout() -> str:
    """Human-readable layout, so the spec and the code cannot drift apart."""
    lines = [
        f"magic            u32  0x{MAGIC:08X}",
        "version          u8",
        "flags            u8",
        "sequence         u32",
        "timestamp_utc_ns u64",
        "station_id       u8",
        "trigger_mask     u16",
        "n_hits           u16",
        f"hits             {HIT_SIZE} bytes each: channel u16, tdc_coarse u16, "
        "tdc_fine u16, adc u16, flags u8",
        "crc32            u32",
        f"total            27 + {HIT_SIZE} * n_hits bytes "
        f"(minimum {MIN_PACKET_SIZE} for n_hits=0)",
    ]
    return "\n".join(lines)
