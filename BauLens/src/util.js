export function isNum(v) {
  return typeof v === "number" && Number.isFinite(v);
}

export function fmtNum(value, digits = 2) {
  if (!isNum(value)) return String(value);
  let s = value.toFixed(digits);
  if (s.includes(".")) s = s.replace(/0+$/, "").replace(/\.$/, "");
  return s.replace(".", ",");
}
