// Fake Modbus TCP PLC for ATA gateway E2E testing (closed-loop hardware mode).
// Register map (signed int16, scaled):
//   addr 0 = internal temp  2000 -> 20.0 °C   (above t_max -> Emergency)
//   addr 2 = ambient temp   2500 -> 25.0 °C
//   addr 4 = grid frequency 4998 -> 49.98 Hz  (outside deadband -> GridBalancing)
// FC05 coil writes (compressor / alarm) are echoed and logged to stdout.
const net = require('net');
const PORT = Number(process.argv[2] || 1502);
const server = net.createServer((sock) => {
  sock.on('data', (buf) => {
    if (buf.length < 8) return;
    const tx = buf.readUInt16BE(0);
    const unit = buf[6];
    const fc = buf[7];
    if (fc === 0x03) {
      const addr = buf.readUInt16BE(8);
      const val = addr === 0 ? 2000 : addr === 2 ? 2500 : addr === 4 ? 4998 : 0;
      const resp = Buffer.from([tx >> 8, tx & 255, 0, 0, 0, 5, unit, 0x03, 2, val >> 8, val & 255]);
      sock.write(resp);
    } else if (fc === 0x05) {
      const resp = Buffer.from([tx >> 8, tx & 255, 0, 0, 0, 6, unit, 0x05, buf[8], buf[9], buf[10], buf[11]]);
      sock.write(resp);
      process.stdout.write(`COIL addr=${buf.readUInt16BE(8)} ${buf[10] === 0xff ? 'ON' : 'OFF'}\n`);
    }
  });
});
server.listen(PORT, '127.0.0.1', () => console.log(`fake-PLC listening on ${PORT}`));
