import net from "node:net";
import tls from "node:tls";
import { dotStuff } from "./compose.mjs";

export class SmtpError extends Error {
  constructor(message, code) {
    super(message);
    this.name = "SmtpError";
    this.code = code || 0;
  }
}

class LineReader {
  constructor(socket) {
    this.buffer = "";
    this.lines = [];
    this.waiters = [];
    this.error = null;
    socket.on("data", (chunk) => {
      this.buffer += chunk.toString("utf8");
      this.pump();
    });
    socket.on("error", (error) => this.fail(error));
    socket.on("close", () => this.fail(new SmtpError("Verbindung wurde geschlossen")));
  }

  fail(error) {
    this.error = this.error || error;
    const waiters = this.waiters.splice(0);
    waiters.forEach((waiter) => waiter.reject(this.error));
  }

  pump() {
    let index = this.buffer.indexOf("\r\n");
    while (index !== -1) {
      const line = this.buffer.slice(0, index);
      this.buffer = this.buffer.slice(index + 2);
      const waiter = this.waiters.shift();
      if (waiter) waiter.resolve(line);
      else this.lines.push(line);
      index = this.buffer.indexOf("\r\n");
    }
  }

  next() {
    if (this.lines.length > 0) return Promise.resolve(this.lines.shift());
    if (this.error) return Promise.reject(this.error);
    return new Promise((resolve, reject) => this.waiters.push({ resolve, reject }));
  }

  async response() {
    const lines = [];
    while (true) {
      const line = await this.next();
      lines.push(line);
      const match = /^(\d{3})([ -]?)/.exec(line);
      if (match && match[2] !== "-") return { code: Number(match[1]), lines, text: lines.join("\n") };
    }
  }
}

function expect(response, codes) {
  const allowed = Array.isArray(codes) ? codes : [codes];
  if (allowed.includes(response.code)) return response;
  throw new SmtpError("SMTP " + response.code + ": " + response.text, response.code);
}

function connect(host, port, secure, timeoutMs) {
  return new Promise((resolve, reject) => {
    const socket = secure
      ? tls.connect({ host, port, servername: host })
      : net.connect({ host, port });
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new SmtpError("Zeitueberschreitung beim Verbindungsaufbau"));
    }, timeoutMs);
    const ready = secure ? "secureConnect" : "connect";
    socket.once(ready, () => {
      clearTimeout(timer);
      resolve(socket);
    });
    socket.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

function startTls(socket, host, timeoutMs) {
  return new Promise((resolve, reject) => {
    const upgraded = tls.connect({ socket, servername: host });
    const timer = setTimeout(() => {
      upgraded.destroy();
      reject(new SmtpError("Zeitueberschreitung beim STARTTLS"));
    }, timeoutMs);
    upgraded.once("secureConnect", () => {
      clearTimeout(timer);
      resolve(upgraded);
    });
    upgraded.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

function capabilities(response) {
  const found = new Set();
  response.lines.slice(1).forEach((line) => {
    const text = line.replace(/^\d{3}[ -]?/, "").trim();
    const [name] = text.split(/\s+/);
    if (name) found.add(name.toUpperCase());
  });
  return found;
}

export async function sendSmtp({ host, port = 587, secure = false, user, pass, from, to, raw, timeoutMs = 20000, requireStartTls = true }) {
  let socket = await connect(host, port, secure, timeoutMs);
  try {
    socket.setTimeout(timeoutMs, () => socket.destroy(new SmtpError("Zeitueberschreitung im SMTP-Dialog")));
    let reader = new LineReader(socket);
    expect(await reader.response(), 220);

    const helo = "clauselens.local";
    socket.write("EHLO " + helo + "\r\n");
    let caps = capabilities(expect(await reader.response(), 250));

    if (!secure) {
      if (!caps.has("STARTTLS")) {
        if (requireStartTls) throw new SmtpError("Server bietet kein STARTTLS, Versand abgebrochen");
      } else {
        socket.write("STARTTLS\r\n");
        expect(await reader.response(), 220);
        socket.removeAllListeners("data");
        socket = await startTls(socket, host, timeoutMs);
        socket.setTimeout(timeoutMs, () => socket.destroy(new SmtpError("Zeitueberschreitung im SMTP-Dialog")));
        reader = new LineReader(socket);
        socket.write("EHLO " + helo + "\r\n");
        caps = capabilities(expect(await reader.response(), 250));
      }
    }

    if (user) {
      const credentials = Buffer.from("\u0000" + user + "\u0000" + (pass || ""), "utf8").toString("base64");
      socket.write("AUTH PLAIN " + credentials + "\r\n");
      expect(await reader.response(), [235, 503]);
    }

    socket.write("MAIL FROM:<" + from + ">\r\n");
    expect(await reader.response(), 250);
    socket.write("RCPT TO:<" + to + ">\r\n");
    expect(await reader.response(), [250, 251]);
    socket.write("DATA\r\n");
    expect(await reader.response(), 354);
    socket.write(dotStuff(raw) + "\r\n.\r\n");
    expect(await reader.response(), 250);
    socket.write("QUIT\r\n");
    socket.end();
    return { ok: true };
  } catch (error) {
    socket.destroy();
    throw error;
  }
}
