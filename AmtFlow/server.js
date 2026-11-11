const http = require('http');
const fs = require('fs');
const path = require('path');
const https = require('https');

const PORT = 3000;
const ROOT = __dirname;

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.ico': 'image/x-icon',
  '.svg': 'image/svg+xml'
};

const SECURITY_HEADERS = {
  'X-Content-Type-Options': 'nosniff',
  'X-Frame-Options': 'DENY',
  'X-XSS-Protection': '1; mode=block',
  'Referrer-Policy': 'strict-origin-when-cross-origin'
};

function callGeminiVision(apiKey, base64Image, mimeType, targetLang, callback) {
  const prompt = `You are AmtFlow, an expert German administrative and civic document AI.
Analyze the attached image of this German official document/letter.
Translate and explain everything accurately in target language: "${targetLang}".
Return ONLY a valid JSON object matching this exact schema:
{
  "agency": "Exact issuing authority (e.g. Deutsche Rentenversicherung, Bundesdruckerei, Bürgeramt, Finanzamt, Ausländerbehörde, Jobcenter)",
  "title": "Specific document title/subject in ${targetLang}",
  "meaning": "Clear, friendly, accurate multi-sentence explanation in ${targetLang} explaining what this letter means, why they received it, and what it confirms or requires.",
  "aktenzeichen": "Extracted Aktenzeichen, VSNR, Steuer-ID, PIN, or Reference (or '—')",
  "deadline": "Extracted deadline in DD.MM.YYYY format (or null if no deadline)",
  "isInformational": true or false,
  "actions": [
    "Concrete step 1 in ${targetLang}",
    "Concrete step 2 in ${targetLang}",
    "Concrete step 3 in ${targetLang}"
  ]
}`;

  const requestData = JSON.stringify({
    contents: [{
      parts: [
        { text: prompt },
        {
          inline_data: {
            mime_type: mimeType || 'image/jpeg',
            data: base64Image
          }
        }
      ]
    }],
    generationConfig: {
      responseMimeType: 'application/json'
    }
  });

  const url = `https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=${apiKey}`;
  const parsedUrl = new URL(url);

  const options = {
    hostname: parsedUrl.hostname,
    port: 443,
    path: parsedUrl.pathname + parsedUrl.search,
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Content-Length': Buffer.byteLength(requestData)
    }
  };

  const req = https.request(options, (res) => {
    let body = '';
    res.on('data', chunk => body += chunk);
    res.on('end', () => {
      try {
        const json = JSON.parse(body);
        if (json.candidates && json.candidates[0] && json.candidates[0].content) {
          const text = json.candidates[0].content.parts[0].text;
          const parsed = JSON.parse(text);
          callback(null, parsed);
        } else {
          callback(new Error(body));
        }
      } catch (err) {
        callback(err);
      }
    });
  });

  req.on('error', err => callback(err));
  req.write(requestData);
  req.end();
}

const server = http.createServer((req, res) => {
  // Health probe endpoint for production SRE / Load balancer
  if (req.url === '/healthz' || req.url === '/readyz') {
    res.writeHead(200, {
      'Content-Type': 'application/json',
      ...SECURITY_HEADERS
    });
    res.end(JSON.stringify({ status: 'healthy', timestamp: Date.now(), service: 'amtflow' }));
    return;
  }

  // Handle Vision AI API
  if (req.method === 'POST' && req.url === '/api/analyze') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        const data = JSON.parse(body);
        const apiKey = data.apiKey || process.env.GEMINI_API_KEY || process.env.API_KEY;
        const targetLang = data.language || 'de';
        
        let rawImage = data.image || '';
        let mimeType = 'image/jpeg';
        if (rawImage.startsWith('data:')) {
          const parts = rawImage.split(',');
          const match = parts[0].match(/:(.*?);/);
          if (match) mimeType = match[1];
          rawImage = parts[1];
        }

        if (!apiKey) {
          res.writeHead(400, {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': '*',
            ...SECURITY_HEADERS
          });
          res.end(JSON.stringify({ success: false, error: 'NO_API_KEY', message: 'Kein API-Key hinterlegt. Bitte API-Key in den Einstellungen hinterlegen.' }));
          return;
        }

        callGeminiVision(apiKey, rawImage, mimeType, targetLang, (err, result) => {
          if (err) {
            console.error('Vision API Error:', err.message);
            res.writeHead(500, {
              'Content-Type': 'application/json',
              'Access-Control-Allow-Origin': '*',
              ...SECURITY_HEADERS
            });
            res.end(JSON.stringify({ success: false, error: 'API_ERROR', message: err.message }));
            return;
          }

          if (result && result.deadline) {
            try {
              const p = result.deadline.split('.');
              const target = new Date(parseInt(p[2]), parseInt(p[1]) - 1, parseInt(p[0]));
              const now = new Date();
              const diff = Math.ceil((target - now) / (1000 * 60 * 60 * 24));
              result.daysRemaining = diff >= 0 ? diff : 0;
            } catch (e) {}
          }

          res.writeHead(200, {
            'Content-Type': 'application/json',
            'Access-Control-Allow-Origin': '*',
            ...SECURITY_HEADERS
          });
          res.end(JSON.stringify({ success: true, data: result }));
        });
      } catch (err) {
        res.writeHead(400, {
          'Content-Type': 'application/json',
          'Access-Control-Allow-Origin': '*',
          ...SECURITY_HEADERS
        });
        res.end(JSON.stringify({ success: false, error: 'INVALID_REQUEST' }));
      }
    });
    return;
  }

  // Serve static files
  let file = req.url.split('?')[0];
  if (file === '/' || file === '') file = '/index.html';
  
  let filePath = path.join(ROOT, file);
  if (!fs.existsSync(filePath) && fs.existsSync(path.join(ROOT, 'public', file))) {
    filePath = path.join(ROOT, 'public', file);
  }

  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404, {
        'Content-Type': 'text/plain; charset=utf-8',
        ...SECURITY_HEADERS
      });
      res.end('404 Not Found');
      return;
    }
    const ext = path.extname(filePath);
    const contentType = MIME[ext] || 'application/octet-stream';
    res.writeHead(200, {
      'Content-Type': contentType,
      'Access-Control-Allow-Origin': '*',
      'Cache-Control': ext === '.json' ? 'public, max-age=3600' : 'no-cache',
      ...SECURITY_HEADERS
    });
    res.end(data);
  });
});

server.listen(PORT, '0.0.0.0', () => {
  console.log(`AmtFlow Production Server running on http://0.0.0.0:${PORT}`);
});
