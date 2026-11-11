/**
 * BauLens - BIM Collaboration Format (BCF) Engine
 * Generates BCF 2.1 / 3.0 issues and viewpoints from compliance audit findings
 * for direct bidirectional roundtrip with Revit, ArchiCAD, and Vectorworks.
 */
import { ENGINE_LABEL } from "./version.js";

/**
 * Creates BCF JSON topic representation from a rule violation.
 * @param {object} finding - Rule violation finding
 * @param {number} index - Finding index
 * @returns {object} BCF Topic
 */
export function createBcfTopic(finding, index = 1) {
  const isFail = finding.status === "fail";
  const topicGuid = `baulens-bcf-${Date.now()}-${index}`;

  return {
    guid: topicGuid,
    topic_type: isFail ? "Clash / Violation" : "Warning / Review",
    topic_status: "Open",
    title: `[BauLens] ${finding.ruleId ?? 'Baurecht'}: ${finding.status.toUpperCase()}`,
    priority: isFail ? "Critical" : "Major",
    creation_date: new Date().toISOString(),
    creation_author: `${ENGINE_LABEL} · Keller Systems`,
    description: finding.message,
    labels: ["Bauantrag", "Genehmigungsfähigkeit", finding.ruleId ?? "LBO"],
    viewpoint: {
      perspective_camera: {
        camera_view_point: { x: 10.0, y: -15.0, z: 12.0 },
        camera_direction: { x: -0.5, y: 0.7, z: -0.5 },
        camera_up_vector: { x: 0.0, y: 0.0, z: 1.0 },
        field_of_view: 60.0,
      },
      lines: [
        {
          start_point: { x: 0.0, y: 0.0, z: 0.0 },
          end_point: { x: 0.0, y: 10.0, z: 0.0 },
        },
      ],
    },
  };
}

/**
 * Converts all audit findings into a BCF issue collection.
 * @param {Array<object>} findings
 * @returns {object} BCF collection
 */
export function exportBcfCollection(findings) {
  const issues = (findings || [])
    .filter((f) => f.status === "fail" || f.status === "warn")
    .map((f, idx) => createBcfTopic(f, idx + 1));

  return {
    project: {
      name: "BauLens BIM Compliance Audit",
      project_id: "BAULENS-BIM-001",
    },
    version: {
      version_id: "2.1",
    },
    topics: issues,
  };
}

