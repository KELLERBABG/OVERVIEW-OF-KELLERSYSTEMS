"""Detector geometry loaded from ``config/detector_geometry.yaml``.

The geometry configuration used to be read by no code at all: the plane positions
and the channel map lived only in prose and every module hardcoded its own idea of
where the planes were. That is how the documentation ended up quoting a 600 mm
lever arm for an angular-resolution budget while the planes it describes sit
140 mm apart.

This module is the single source of truth. It reads the YAML, validates that the
channel map is consistent and that the sides are mirrored, and exposes the
quantities the simulation needs -- in particular the *per-orientation lever arm*,
which is the distance between the two layers that actually measure the same
coordinate, and therefore the distance that sets angular resolution.
"""
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np
import yaml

# <repo>/src/inversion/muonscope/simulation/geometry.py -> <repo>
_REPO_ROOT = Path(__file__).resolve().parents[4]
DEFAULT_GEOMETRY_PATH = str(_REPO_ROOT / "config" / "detector_geometry.yaml")


@dataclass(frozen=True)
class StripPlane:
    """One strip layer measuring a single coordinate."""

    plane_id: str
    orientation: str
    num_strips: int
    strip_width_mm: float
    strip_height_mm: float
    z_mm: float
    sipm_channels: Tuple[int, int]

    @property
    def active_width_mm(self) -> float:
        return self.num_strips * self.strip_width_mm

    @property
    def channel_ids(self) -> np.ndarray:
        return np.arange(self.sipm_channels[0], self.sipm_channels[1] + 1)


@dataclass(frozen=True)
class Panel:
    """A tracker station: two coordinate layers at two z positions."""

    panel_id: str
    active_area_mm: Tuple[float, float]
    z_offset_mm: float
    planes: List[StripPlane]
    plane_separation_mm: float

    def planes_for(self, orientation: str) -> List[StripPlane]:
        return [p for p in self.planes if p.orientation == orientation]

    def lever_arm_mm(self, orientation: str) -> float:
        """Distance between the two layers measuring ``orientation``.

        This is the arm that sets the angular resolution of the station:
        ``sigma_theta = sqrt(2) * sigma_pos / lever_arm``. Quoting the panel-to-panel
        separation here instead inflates the resolution by that ratio.
        """
        zs = [p.z_mm for p in self.planes_for(orientation)]
        if len(zs) < 2:
            raise ValueError(
                f"panel {self.panel_id} has {len(zs)} {orientation} plane(s); "
                "two are required to measure a direction"
            )
        return float(abs(max(zs) - min(zs)))

    def outer_z_mm(self) -> float:
        """The plane furthest from the target volume, i.e. the entry face."""
        return float(max(abs(p.z_mm) for p in self.planes))


@dataclass(frozen=True)
class DetectorGeometry:
    name: str
    version: str
    units: str
    panels: Dict[str, Panel]
    coincidence_window_ns: float = 8.0
    max_chi2_ndf: float = 2.5
    raw: Dict = field(default_factory=dict, repr=False)

    # -- convenience accessors -------------------------------------------------

    @property
    def top(self) -> Panel:
        return self.panels["top_panel"]

    @property
    def bottom(self) -> Panel:
        return self.panels["bottom_panel"]

    def bank_span(self, key: str) -> Tuple[int, int]:
        """Channel range covered by one panel's readout bank."""
        ranges = [p.sipm_channels for p in self.panels[key].planes]
        return int(min(r[0] for r in ranges)), int(max(r[1] for r in ranges))

    def bank_size(self, key: str) -> int:
        lo, hi = self.bank_span(key)
        return hi - lo + 1

    def channel_span(self) -> Tuple[int, int]:
        lo = min(self.bank_span(k)[0] for k in self.panels)
        hi = max(self.bank_span(k)[1] for k in self.panels)
        return int(lo), int(hi)

    def total_channels(self) -> int:
        """Total strip channels across all planes, all banks.

        This is the number of SiPM channels the instrument needs, and it is the
        number the documentation disagrees with itself about: the BOM says 128,
        the geometry says 8 planes x 64 = 512, and the prose says 256.
        """
        return sum(p.num_strips for panel in self.panels.values() for p in panel.planes)

    def total_strips(self) -> int:
        return self.total_channels()

    def bank_offsets(self) -> Dict[str, int]:
        """Global channel offset of each panel's bank.

        The shipped geometry numbers both panels 0-255, which is only coherent if
        each panel has its own readout bank -- the same physical arrangement as a
        two-board front end. Baked into a *global* channel id here so the
        simulation cannot silently alias a top-panel strip onto a bottom-panel one.
        """
        offsets: Dict[str, int] = {}
        cursor = 0
        for key in sorted(self.panels):
            lo, _hi = self.bank_span(key)
            offsets[key] = cursor - lo
            cursor += self.bank_size(key)
        return offsets

    def global_channel_id(self, panel_key: str, local_channel: int) -> int:
        return int(local_channel + self.bank_offsets()[panel_key])

    def entry_exit_z_mm(self) -> Tuple[float, float]:
        """z of the outermost top and bottom layers, in mm.

        A ray tracer must launch and stop its tracks here, otherwise the track
        endpoints do not coincide with the outer measurement layers and the
        reconstructed direction is biased.
        """
        return self.top.outer_z_mm(), -self.bottom.outer_z_mm()

    def panel_centre_separation_mm(self) -> float:
        return abs(self.top.z_offset_mm - self.bottom.z_offset_mm)

    def summary(self) -> Dict[str, object]:
        return {
            "name": self.name,
            "total_channels": self.total_channels(),
            "banks": {k: self.bank_span(k) for k in sorted(self.panels)},
            "bank_offsets": self.bank_offsets(),
            "top_lever_arm_x_mm": self.top.lever_arm_mm("X"),
            "top_lever_arm_y_mm": self.top.lever_arm_mm("Y"),
            "bottom_lever_arm_x_mm": self.bottom.lever_arm_mm("X"),
            "bottom_lever_arm_y_mm": self.bottom.lever_arm_mm("Y"),
            "panel_centre_separation_mm": self.panel_centre_separation_mm(),
            "entry_exit_z_mm": self.entry_exit_z_mm(),
        }


def _parse_plane(entry: Dict) -> StripPlane:
    sipm = entry.get("sipm_channels", [0, entry["num_strips"] - 1])
    if len(sipm) != 2:
        raise ValueError(f"plane {entry.get('plane_id')}: sipm_channels must be [lo, hi]")
    return StripPlane(
        plane_id=str(entry["plane_id"]),
        orientation=str(entry["orientation"]).upper(),
        num_strips=int(entry["num_strips"]),
        strip_width_mm=float(entry["strip_width"]),
        strip_height_mm=float(entry.get("strip_height", 0.0)),
        z_mm=float(entry["z_position"]),
        sipm_channels=(int(sipm[0]), int(sipm[1])),
    )


def _parse_panel(entry: Dict) -> Panel:
    planes = [_parse_plane(p) for p in entry["planes"]]
    return Panel(
        panel_id=str(entry["id"]),
        active_area_mm=(float(entry["active_area"][0]), float(entry["active_area"][1])),
        z_offset_mm=float(entry["z_offset"]),
        planes=planes,
        plane_separation_mm=float(entry.get("plane_separation", 0.0)),
    )


def parse_detector_geometry(data: Dict) -> DetectorGeometry:
    """Build and validate a :class:`DetectorGeometry` from parsed YAML."""
    panels_cfg = data.get("panels", {})
    if "top_panel" not in panels_cfg or "bottom_panel" not in panels_cfg:
        raise ValueError("geometry requires both top_panel and bottom_panel")

    panels: Dict[str, Panel] = {}
    for key in ("top_panel", "bottom_panel"):
        panels[key] = _parse_panel(panels_cfg[key])

    geo = DetectorGeometry(
        name=str(data.get("system", {}).get("name", "unnamed")),
        version=str(data.get("system", {}).get("version", "0")),
        units=str(data.get("system", {}).get("units", "millimeter")),
        panels=panels,
        coincidence_window_ns=float(
            data.get("trigger_logic", {}).get("coincidence_window_ns", 8.0)),
        max_chi2_ndf=float(data.get("trigger_logic", {}).get("max_chi2_ndf", 2.5)),
        raw=data,
    )
    validate_detector_geometry(geo)
    return geo


def validate_detector_geometry(geo: DetectorGeometry) -> None:
    """Raise on geometry that cannot measure a direction.

    Checks, in order: both coordinate orientations are present on both stations
    with two layers each; the channel map is contiguous and non-overlapping; and
    the stations sit on opposite sides of the target volume.
    """
    problems: List[str] = []

    for key, panel in geo.panels.items():
        for orientation in ("X", "Y"):
            planes = panel.planes_for(orientation)
            if len(planes) != 2:
                problems.append(
                    f"{key}/{orientation}: expected exactly 2 layers, found {len(planes)}")
                continue
            zs = sorted(p.z_mm for p in planes)
            if zs[0] == zs[1]:
                problems.append(f"{key}/{orientation}: both layers at z={zs[0]} mm")
            widths = {p.active_width_mm for p in planes}
            if len(widths) > 1:
                problems.append(
                    f"{key}/{orientation}: layers have different active widths {sorted(widths)}")

    # Channel map: each panel owns a readout bank, and within a bank the plane
    # ranges must tile the range exactly once with no gap and no overlap. Two
    # panels sharing the same local numbering is legitimate (separate boards) and
    # is disambiguated by bank_offsets(); a gap *within* a bank is not.
    for key, panel in sorted(geo.panels.items()):
        ranges = sorted((p.sipm_channels, p.plane_id) for p in panel.planes)
        cursor: Optional[int] = None
        for (lo, hi), plane_id in ranges:
            if hi < lo:
                problems.append(f"{key}/{plane_id}: sipm_channels reversed ({lo}, {hi})")
                continue
            if hi - lo + 1 != next(
                    p.num_strips for p in panel.planes if p.plane_id == plane_id):
                problems.append(
                    f"{key}/{plane_id}: channel range covers {hi - lo + 1} ids "
                    f"for {next(p.num_strips for p in panel.planes if p.plane_id == plane_id)} strips")
            if cursor is None:
                cursor = hi + 1
            elif lo != cursor:
                problems.append(
                    f"{key}/{plane_id}: channel map gap or overlap at {lo} "
                    f"(expected {cursor}) within the {key} bank")
                cursor = hi + 1
            else:
                cursor = hi + 1

    if geo.top.z_offset_mm <= 0.0 or geo.bottom.z_offset_mm >= 0.0:
        problems.append("top_panel must have z_offset > 0 and bottom_panel z_offset < 0")

    if problems:
        raise ValueError("invalid detector geometry:\n  - " + "\n  - ".join(problems))


def load_detector_geometry(path: str = DEFAULT_GEOMETRY_PATH) -> DetectorGeometry:
    """Read, parse and validate the detector geometry YAML."""
    with open(path, "r", encoding="utf-8") as handle:
        data = yaml.safe_load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: expected a YAML mapping at the top level")
    return parse_detector_geometry(data)
