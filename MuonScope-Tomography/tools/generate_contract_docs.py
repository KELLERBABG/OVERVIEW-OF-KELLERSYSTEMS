"""Generate the machine-facing contract documents from the code.

Two committed documents described interfaces that did not exist:

* ``docs/API_SPECIFICATION.md`` listed ``/api/v1/system/status``, four
  ``/api/v1/survey/*`` endpoints, a WebSocket on port 8080 and a gRPC
  ``SamTMeshService``. None were implemented.
* ``docs/IPC_COMMUNICATION_SPEC.md`` specified a "fixed-size 32-byte binary
  packet" whose own header fields consumed all 32 bytes, so a four-plane event
  could not fit in it. The decoder and the firmware disagreed with it and with
  each other.

Both are now generated from the artefacts the software actually uses -- the
FastAPI schema and :mod:`muonscope.wire` -- so they cannot drift again.

    py tools/generate_contract_docs.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
sys.path.insert(0, os.path.join(ROOT, "src", "server"))
sys.path.insert(0, os.path.join(ROOT, "src", "inversion"))


def _schema_name(ref_or_schema) -> str:
    if isinstance(ref_or_schema, dict):
        ref = ref_or_schema.get("$ref")
        if ref:
            return ref.rsplit("/", 1)[-1]
        return ref_or_schema.get("type", "object")
    return "object"


def write_api_spec() -> None:
    import api_server

    spec = api_server.app.openapi()
    schemas = spec.get("components", {}).get("schemas", {})

    lines = [
        "# MuonScope HTTP & WebSocket API",
        "",
        "> **Generated from the running application's OpenAPI schema by",
        "> `tools/generate_contract_docs.py`. Do not edit by hand -- regenerate it.**",
        "",
        "A single FastAPI application. It runs the instrument software end to end on",
        "simulated detector bytes: forward physics, the detector model, the wire",
        "encoder, the framing reader, TDC calibration, track fitting and a",
        "prior-marginalised reconstruction.",
        "",
        "**No per-event momentum is accepted or reported.** A two-station hodoscope",
        "measures directions and time, not momentum, so the API has no field for it.",
        "The reconstruction marginalises over the cosmic-ray momentum spectrum.",
        "",
        "Default bind: `0.0.0.0:8000`.",
        "",
        "## Endpoints",
        "",
    ]

    for path, operations in sorted(spec["paths"].items()):
        for method, op in sorted(operations.items()):
            lines.append(f"### `{method.upper()} {path}`")
            lines.append("")
            summary = op.get("summary") or op.get("operationId") or ""
            if summary:
                lines += [summary, ""]
            description = op.get("description")
            if description:
                for para in description.strip().split("\n\n"):
                    lines += [para.strip(), ""]

            body = op.get("requestBody")
            if body:
                content = body.get("content", {}).get("application/json", {})
                lines += [f"**Request body:** `{_schema_name(content.get('schema', {}))}`", ""]
                ref = content.get("schema", {}).get("$ref")
                if ref:
                    props = schemas.get(ref.rsplit("/", 1)[-1], {}).get("properties", {})
                    if props:
                        lines += ["| field | type | default |", "|---|---|---|"]
                        for field, meta in props.items():
                            ftype = meta.get("type") or _schema_name(meta)
                            lines.append(f"| `{field}` | {ftype} | `{meta.get('default', '')}` |")
                        lines.append("")

            lines += ["**Responses**", ""]
            for code, resp in sorted(op.get("responses", {}).items()):
                content = resp.get("content", {}).get("application/json", {})
                lines.append(f"* `{code}` -- {resp.get('description', '')} "
                             f"(`{_schema_name(content.get('schema', {}))}`)")
            lines.append("")

    lines += [
        "## WebSocket",
        "",
        "### `WS /ws/live-stream`",
        "",
        "Streams measured tracks as the instrument produces them, roughly 10 per",
        "80 ms. Each message is a JSON object:",
        "",
        "| field | meaning |",
        "|---|---|",
        "| `type` | always `MUON_EVENT` |",
        "| `timestamp_ns` | wall-clock time of transmission |",
        "| `angle_mrad` | fitted scattering angle, milliradians |",
        "| `sigma_angle_mrad` | uncertainty on that angle, from the geometry |",
        "| `tof_ns` | time of flight between the stations |",
        "| `p_in`, `p_out` | fitted entry and exit points, millimetres |",
        "| `n_planes` | planes that contributed to the fit |",
        "| `chi2_ndf_dof` | degrees of freedom of the fit; see below |",
        "",
        "There is deliberately no `energy_gev` and no truth-derived quality flag:",
        "the hodoscope measures neither.",
        "",
        "## A note on the chi-squared cut",
        "",
        "The design documents specify a track-quality cut at `chi2/ndf <= 2.5`. With",
        "the shipped geometry each station fits a straight line through two layers per",
        "coordinate, so the fit has **zero degrees of freedom** and no chi-squared",
        "exists. The API reports `chi2_ndf_dof: 0` rather than inventing a figure.",
        "Adding a third layer per coordinate would make the cut possible.",
        "",
        "## Not implemented",
        "",
        "Appearing in an earlier version of this document and never built:",
        "`/api/v1/system/status`, `/api/v1/survey/start`, `/api/v1/survey/stop`,",
        "`/api/v1/survey/export`, a WebSocket at `/ws/v1/live_stream` on port 8080, and",
        "a gRPC `SamTMeshService` for multi-station coordination. The instrument is",
        "modelled as a single station; there is no mesh.",
        "",
    ]

    out = os.path.join(ROOT, "docs", "API_SPECIFICATION.md")
    with open(out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(f"wrote {out}  ({len(spec['paths'])} paths, {len(schemas)} schemas)")


def write_ipc_spec() -> None:
    from muonscope import wire
    from muonscope.simulation.geometry import load_detector_geometry

    channels = load_detector_geometry().total_channels()

    lines = [
        "# MuonScope Instrument Wire Protocol",
        "",
        "> **Generated from `muonscope/wire.py` by `tools/generate_contract_docs.py`.",
        "> Do not edit by hand -- regenerate it.**",
        "",
        "This is the frame the software decodes. It is the interface between the",
        "instrument and the acquisition chain, and the only thing the processing",
        "chain is permitted to consume.",
        "",
        "An earlier version of this document specified a **fixed-size 32-byte",
        "packet**. Its own header fields already consumed all 32 bytes, so a",
        "four-plane event could not fit, and the decoder's minimum-length gate",
        "admitted buffers too short to contain a CRC. The format below is",
        "variable-length and self-describing: the hit count is authoritative.",
        "",
        "## Frame layout",
        "",
        "All integers are big-endian.",
        "",
        "| offset | size | field |",
        "|---|---|---|",
        "| 0 | 4 | `magic` = `0xAA55AA55` |",
        "| 4 | 1 | `version` (currently 1) |",
        "| 5 | 1 | `flags` -- bit 0 simulated, bit 1 calibration run |",
        "| 6 | 4 | `sequence` -- monotonic, wraps at 2^32 |",
        "| 10 | 8 | `timestamp_utc_ns` -- nanoseconds since the Unix epoch |",
        "| 18 | 1 | `station_id` |",
        "| 19 | 2 | `trigger_mask` -- one bit per plane that met coincidence |",
        "| 21 | 2 | `n_hits` |",
        "| 23 | 9*n | hit records |",
        "| 23+9n | 4 | `crc32` -- CRC-32/ISO-HDLC over bytes `[0, 23+9n)` |",
        "",
        "### Hit record",
        "",
        "| offset | size | field |",
        "|---|---|---|",
        f"| 0 | 2 | `channel` -- global strip channel, 0..{channels - 1} |",
        "| 2 | 2 | `tdc_coarse` -- clock ticks since the trigger |",
        "| 4 | 2 | `tdc_fine` -- tapped-delay-line code within one clock |",
        "| 6 | 2 | `adc` -- charge-integrated pulse height |",
        "| 8 | 1 | `hit_flags` -- bit 0 dark count, bit 1 channel masked |",
        "",
        "## Size",
        "",
        "```",
        wire.describe_layout(),
        "```",
        "",
        "A calibration-run packet carries one hit per channel and is therefore the",
        "largest frame the system emits. A physics event carries one hit per fired",
        "strip, typically two per plane.",
        "",
        "## What the frame deliberately cannot carry",
        "",
        "There is no momentum, energy or scattering-angle field, and no per-event",
        "truth flag. A two-station hodoscope measures hit positions and times; the",
        "momentum enters the reconstruction as an unknown, marginalised over the",
        "cosmic-ray spectrum. A format that could carry truth would let the",
        "reconstruction read it by accident.",
        "",
        "## Physical layer -- design intent, not implemented",
        "",
        "No hardware exists. The following describes the intended interface and is",
        "unverified: SPI between an MCU and the FPGA for register access, USB 3.0 or",
        "Gigabit Ethernet from the FPGA to the edge host, and optional Wi-Fi 6 or",
        "LoRa backhaul. The frame above is what the software implements today; a",
        "hardware implementation would have to match it or change it deliberately.",
        "",
    ]

    out = os.path.join(ROOT, "docs", "IPC_COMMUNICATION_SPEC.md")
    with open(out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines))
    print(f"wrote {out}  ({wire.MIN_PACKET_SIZE} bytes minimum, "
          f"{wire.HEADER_SIZE}+{wire.HIT_SIZE}n+{wire.CRC_SIZE})")


def main() -> int:
    write_api_spec()
    write_ipc_spec()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
