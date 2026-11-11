"""DICOM export.

What this replaced
------------------
The previous exporter wrote a raw ``uint16`` array with ``ndarray.tofile`` and a
``.dcm`` extension. The files had no 128-byte preamble, no ``DICM`` magic, no
file meta group and no data elements at all, while the project's documentation
described a "multi-slice DICOM series with HU tags" and the audit reported it as
a pass. ``pydicom`` is declared in ``pyproject.toml`` and was never imported.

What this writes
----------------
Part 10 files, Explicit VR Little Endian, with a file meta group and the CT Image
IOD attributes a viewer needs to place and scale the slices. Written by hand so
the export does not depend on an optional dependency being installed.

One deliberate honesty point: the pixel values are **scattering density**, not
Hounsfield units. HU is defined for X-ray linear attenuation and this modality
does not measure it, so inventing HU tags would be a fabricated result. The
``RescaleSlope``/``RescaleIntercept`` pair instead maps the stored integers back
to ``lambda`` in cm^-1 exactly, and the series description says so.
"""
from __future__ import annotations

import hashlib
import os
import struct
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np

# VRs that use the 12-byte long form with a 4-byte length.
_LONG_VRS = {"OB", "OW", "OF", "SQ", "UT", "UN", "OD", "OL", "OV", "UC", "UR"}
# VRs whose values are written as text and padded with a space.
_TEXT_VRS = {"AE", "AS", "CS", "DA", "DS", "DT", "IS", "LO", "LT", "PN",
             "SH", "ST", "TM", "UC", "UR", "UT"}

CT_IMAGE_STORAGE = "1.2.840.10008.5.1.4.1.1.2"
EXPLICIT_VR_LITTLE_ENDIAN = "1.2.840.10008.1.2.1"
IMPLEMENTATION_CLASS_UID = "1.2.826.0.1.3680043.8.498.1"
UID_ROOT = "1.2.826.0.1.3680043.8.498.20261007"


class DicomError(ValueError):
    """Raised when a buffer is not a valid DICOM Part 10 file."""


def _pad(value: bytes, vr: str) -> bytes:
    if len(value) % 2 == 0:
        return value
    return value + (b"\x00" if vr in ("UI", "OB", "OW") else b" ")


def _encode_value(vr: str, value) -> bytes:
    """Encode a python value into the bytes DICOM expects for that VR."""
    if vr in ("US", "SS", "UL", "SL"):
        fmt = {"US": "H", "SS": "h", "UL": "I", "SL": "i"}[vr]
        items = value if isinstance(value, (list, tuple, np.ndarray)) else [value]
        return struct.pack("<" + fmt * len(items), *[int(v) for v in items])
    if vr in ("FL", "FD"):
        fmt = "f" if vr == "FL" else "d"
        items = value if isinstance(value, (list, tuple, np.ndarray)) else [value]
        return struct.pack("<" + fmt * len(items), *[float(v) for v in items])
    if vr in ("OB", "OW", "UN"):
        return _pad(bytes(value), vr)
    if vr == "AT":
        return struct.pack("<HH", int(value[0]), int(value[1]))
    # Text VRs: numbers are rendered first (DS/IS are decimal/integer strings).
    if isinstance(value, (list, tuple, np.ndarray)):
        text = "\\".join(
            f"{v:.10g}" if isinstance(v, (float, np.floating)) else str(v) for v in value)
    elif isinstance(value, (float, np.floating)):
        text = f"{float(value):.10g}"
    else:
        text = str(value)
    return _pad(text.encode("ascii", errors="strict"), vr)


def _element(tag: Tuple[int, int], vr: str, value) -> bytes:
    payload = _encode_value(vr, value)
    tag_bytes = struct.pack("<HH", tag[0], tag[1])
    if vr in _LONG_VRS:
        return tag_bytes + vr.encode("ascii") + b"\x00\x00" + struct.pack("<I", len(payload)) + payload
    return tag_bytes + vr.encode("ascii") + struct.pack("<H", len(payload)) + payload


def deterministic_uid(*parts: str) -> str:
    """A reproducible UID, so re-running an export does not mint new identities."""
    digest = hashlib.sha256("|".join(parts).encode("utf-8")).hexdigest()
    number = int(digest[:24], 16)
    return f"{UID_ROOT}.{number}"


def _spacing_ok(spacing_mm: Sequence[float]) -> Tuple[float, float, float]:
    if len(spacing_mm) != 3:
        raise ValueError("spacing_mm must have three components")
    values = tuple(float(s) for s in spacing_mm)
    if any(s <= 0.0 for s in values):
        raise ValueError(f"spacing_mm must be positive, got {values}")
    return values


def volume_to_pixels(volume: np.ndarray) -> Tuple[np.ndarray, float, float]:
    """Scale a float volume to uint16 and return the exact inverse mapping.

    Returns ``(pixels, slope, intercept)`` such that
    ``volume == pixels * slope + intercept`` to within float32 rounding. Storing
    the mapping rather than assuming one keeps the physical units recoverable, so
    a reader can never mistake the integers for Hounsfield units.
    """
    vol = np.asarray(volume, dtype=np.float64)
    v_min = float(vol.min())
    v_max = float(vol.max())
    if not np.isfinite(v_min) or not np.isfinite(v_max):
        raise ValueError("volume contains non-finite values")
    if v_max <= v_min:
        return np.zeros(vol.shape, dtype=np.uint16), 1.0, v_min
    slope = (v_max - v_min) / 65535.0
    pixels = np.round((vol - v_min) / slope).astype(np.uint16)
    return pixels, slope, v_min


def build_dicom_slice(
    pixels: np.ndarray,
    *,
    slope: float,
    intercept: float,
    pixel_spacing_mm: Tuple[float, float],
    slice_thickness_mm: float,
    image_position_mm: Tuple[float, float, float],
    series_uid: str,
    study_uid: str,
    frame_of_reference_uid: str,
    instance_number: int,
    series_number: int = 1,
    series_description: str = "Muon scattering density lambda [1/cm]",
    patient_id: str = "MUONSCOPE^PHANTOM",
    study_date: Optional[str] = None,
    study_time: Optional[str] = None,
) -> bytes:
    """Assemble one Part 10 DICOM file as bytes."""
    rows, cols = int(pixels.shape[0]), int(pixels.shape[1])
    sop_instance_uid = deterministic_uid(series_uid, str(instance_number))
    now = datetime.now(timezone.utc)
    study_date = study_date or now.strftime("%Y%m%d")
    study_time = study_time or now.strftime("%H%M%S")

    # -- file meta group (always Explicit VR Little Endian) -------------------
    meta_body = b"".join([
        _element((0x0002, 0x0001), "OB", b"\x00\x01"),
        _element((0x0002, 0x0002), "UI", CT_IMAGE_STORAGE),
        _element((0x0002, 0x0003), "UI", sop_instance_uid),
        _element((0x0002, 0x0010), "UI", EXPLICIT_VR_LITTLE_ENDIAN),
        _element((0x0002, 0x0012), "UI", IMPLEMENTATION_CLASS_UID),
        _element((0x0002, 0x0013), "SH", "MUONSCOPE_1_0"),
    ])
    meta = _element((0x0002, 0x0000), "UL", len(meta_body)) + meta_body

    # -- dataset --------------------------------------------------------------
    window_centre = (float(pixels.max()) + float(pixels.min())) / 2.0
    window_width = max(float(pixels.max()) - float(pixels.min()), 1.0)

    dataset = b"".join([
        _element((0x0008, 0x0005), "CS", "ISO_IR 100"),
        _element((0x0008, 0x0008), "CS", ["DERIVED", "SECONDARY", "AXIAL"]),
        _element((0x0008, 0x0016), "UI", CT_IMAGE_STORAGE),
        _element((0x0008, 0x0018), "UI", sop_instance_uid),
        _element((0x0008, 0x0020), "DA", study_date),
        _element((0x0008, 0x0021), "DA", study_date),
        _element((0x0008, 0x0023), "DA", study_date),
        _element((0x0008, 0x0030), "TM", study_time),
        _element((0x0008, 0x0031), "TM", study_time),
        _element((0x0008, 0x0033), "TM", study_time),
        _element((0x0008, 0x0060), "CS", "CT"),
        _element((0x0008, 0x0070), "LO", "MuonScope Tomography"),
        _element((0x0008, 0x0080), "LO", "MuonScope"),
        _element((0x0008, 0x103E), "LO", series_description),
        _element((0x0008, 0x1090), "LO", "MuonScope Hodoscope"),
        _element((0x0010, 0x0010), "PN", patient_id),
        _element((0x0010, 0x0020), "LO", "MUONSCOPE-001"),
        _element((0x0018, 0x0050), "DS", slice_thickness_mm),
        _element((0x0018, 0x0088), "DS", slice_thickness_mm),
        _element((0x0018, 0x1020), "LO", "MuonScope 1.0"),
        _element((0x0020, 0x000D), "UI", study_uid),
        _element((0x0020, 0x000E), "UI", series_uid),
        _element((0x0020, 0x0010), "SH", "MUONSCOPE"),
        _element((0x0020, 0x0011), "IS", series_number),
        _element((0x0020, 0x0013), "IS", instance_number),
        _element((0x0020, 0x0032), "DS", [f"{v:.10g}" for v in image_position_mm]),
        _element((0x0020, 0x0037), "DS", ["1", "0", "0", "0", "1", "0"]),
        _element((0x0020, 0x0052), "UI", frame_of_reference_uid),
        _element((0x0028, 0x0002), "US", 1),
        _element((0x0028, 0x0004), "CS", "MONOCHROME2"),
        _element((0x0028, 0x0010), "US", rows),
        _element((0x0028, 0x0011), "US", cols),
        _element((0x0028, 0x0030), "DS", [f"{pixel_spacing_mm[0]:.10g}",
                                          f"{pixel_spacing_mm[1]:.10g}"]),
        _element((0x0028, 0x0100), "US", 16),
        _element((0x0028, 0x0101), "US", 16),
        _element((0x0028, 0x0102), "US", 15),
        _element((0x0028, 0x0103), "US", 0),
        _element((0x0028, 0x1050), "DS", window_centre),
        _element((0x0028, 0x1051), "DS", window_width),
        _element((0x0028, 0x1052), "DS", intercept),
        _element((0x0028, 0x1053), "DS", slope),
        _element((0x7FE0, 0x0010), "OW",
                 np.ascontiguousarray(pixels, dtype="<u2").tobytes()),
    ])

    preamble = b"\x00" * 128
    return preamble + b"DICM" + meta + dataset


def export_to_dicom_series(
    volume: np.ndarray,
    output_dir: str,
    pixel_spacing_mm: Tuple[float, float, float] = (5.0, 5.0, 5.0),
    origin_mm: Tuple[float, float, float] = (0.0, 0.0, 0.0),
    series_description: str = "Muon scattering density lambda [1/cm]",
    study_uid: Optional[str] = None,
    series_uid: Optional[str] = None,
) -> int:
    """Write ``volume`` as a DICOM series, one file per slice along z.

    Returns the number of slices written. The pixel spacing convention is the
    usual DICOM one: ``pixel_spacing_mm`` is ``(row_spacing, column_spacing,
    slice_spacing)``.
    """
    vol = np.asarray(volume)
    if vol.ndim != 3:
        raise ValueError(f"expected a 3-D volume, got shape {vol.shape}")
    spacing = _spacing_ok(pixel_spacing_mm)
    os.makedirs(output_dir, exist_ok=True)

    nx, ny, nz = vol.shape
    pixels_all, slope, intercept = volume_to_pixels(vol)

    seed = hashlib.sha256(np.ascontiguousarray(vol).tobytes()).hexdigest()
    study_uid = study_uid or deterministic_uid("study", seed)
    series_uid = series_uid or deterministic_uid("series", seed)
    frame_uid = deterministic_uid("frame", seed)

    written = 0
    for z in range(nz):
        # DICOM stores rows = y and columns = x for an axial slice.
        slice_pixels = np.ascontiguousarray(pixels_all[:, :, z].T)
        payload = build_dicom_slice(
            slice_pixels,
            slope=slope,
            intercept=intercept,
            pixel_spacing_mm=(spacing[0], spacing[1]),
            slice_thickness_mm=spacing[2],
            image_position_mm=(origin_mm[0], origin_mm[1], origin_mm[2] + z * spacing[2]),
            series_uid=series_uid,
            study_uid=study_uid,
            frame_of_reference_uid=frame_uid,
            instance_number=z + 1,
            series_description=series_description,
        )
        path = os.path.join(output_dir, f"slice_{z:04d}.dcm")
        with open(path, "wb") as handle:
            handle.write(payload)
        written += 1
    return written


# ---------------------------------------------------------------------------
# minimal reader, so an export can be verified without an extra dependency
# ---------------------------------------------------------------------------

@dataclass
class DicomSlice:
    elements: Dict[Tuple[int, int], Tuple[str, object]]
    pixel_data: Optional[np.ndarray]

    def get(self, tag: Tuple[int, int], default=None):
        entry = self.elements.get(tag)
        return entry[1] if entry else default

    def as_physical(self) -> Optional[np.ndarray]:
        """Pixel data mapped back to the physical quantity."""
        if self.pixel_data is None:
            return None
        slope = float(self.get((0x0028, 0x1053), 1.0))
        intercept = float(self.get((0x0028, 0x1052), 0.0))
        return self.pixel_data.astype(np.float64) * slope + intercept


def parse_dicom(buffer: bytes) -> DicomSlice:
    """Parse a Part 10 Explicit VR Little Endian file.

    Only what this module writes is supported, which is the point: the test can
    round-trip the export without trusting a third-party parser to agree.
    """
    if len(buffer) < 132:
        raise DicomError(f"file of {len(buffer)} bytes is shorter than the 132-byte preamble")
    if buffer[128:132] != b"DICM":
        raise DicomError("missing DICM magic at offset 128")
    if len(buffer) < 132 + 8:
        raise DicomError("file has a preamble but no data set")

    offset = 132
    # Skip the meta group; the transfer syntax is explicit VR LE throughout.
    elements: Dict[Tuple[int, int], Tuple[str, object]] = {}
    pixel_data: Optional[np.ndarray] = None

    while offset + 8 <= len(buffer):
        group, element = struct.unpack_from("<HH", buffer, offset)
        vr = buffer[offset + 4:offset + 6].decode("ascii", errors="replace")
        if vr in _LONG_VRS:
            if offset + 12 > len(buffer):
                break
            length = struct.unpack_from("<I", buffer, offset + 8)[0]
            value_offset = offset + 12
        else:
            length = struct.unpack_from("<H", buffer, offset + 6)[0]
            value_offset = offset + 8
        if value_offset + length > len(buffer):
            raise DicomError(
                f"element ({group:04X},{element:04X}) claims {length} bytes past end of file")
        raw = buffer[value_offset:value_offset + length]
        tag = (group, element)

        if tag == (0x7FE0, 0x0010):
            pixel_data = np.frombuffer(raw, dtype="<u2").copy()
        elif vr in ("US",):
            elements[tag] = (vr, list(struct.unpack("<" + "H" * (length // 2), raw)))
        elif vr == "UL":
            elements[tag] = (vr, struct.unpack("<I", raw)[0] if length == 4 else list(raw))
        elif vr in ("OB", "OW", "UN"):
            elements[tag] = (vr, raw)
        else:
            text = raw.decode("ascii", errors="replace").rstrip("\x00 ")
            parts = text.split("\\") if "\\" in text else text
            elements[tag] = (vr, parts)

        offset = value_offset + length

    if not elements and pixel_data is None:
        raise DicomError("no data elements found after the preamble")

    if pixel_data is not None:
        rows = elements.get((0x0028, 0x0010), ("US", [0]))[1]
        cols = elements.get((0x0028, 0x0011), ("US", [0]))[1]
        rows = int(rows[0]) if isinstance(rows, list) else int(rows)
        cols = int(cols[0]) if isinstance(cols, list) else int(cols)
        if rows and cols and pixel_data.size == rows * cols:
            pixel_data = pixel_data.reshape(rows, cols)
        else:
            raise DicomError(
                f"pixel data has {pixel_data.size} samples, expected {rows}x{cols}")

    return DicomSlice(elements=elements, pixel_data=pixel_data)


def read_dicom_series(directory: str) -> List[DicomSlice]:
    """Read every ``*.dcm`` in a directory, ordered by instance number."""
    names = sorted(f for f in os.listdir(directory) if f.lower().endswith(".dcm"))
    slices = []
    for name in names:
        with open(os.path.join(directory, name), "rb") as handle:
            slices.append(parse_dicom(handle.read()))
    slices.sort(key=lambda s: int(s.get((0x0020, 0x0013), 0)))
    return slices
