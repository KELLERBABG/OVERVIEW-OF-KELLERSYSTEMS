# ClauseLens - Production Docker Image
# Zero runtime dependencies, 100% offline, privacy-first AGB/contract audit engine

FROM node:20-alpine

# Set production environment
ENV NODE_ENV=production \
    PORT=3000 \
    HOST=0.0.0.0

WORKDIR /app

# Copy application sources
COPY package.json clauselens.config.json README.md LICENSE ./
COPY bin ./bin
COPY src ./src
COPY index.html ./index.html

# Run as unprivileged user
USER node

# Expose standard port
EXPOSE 3000

# Native healthcheck using Node.js built-in fetch (zero dependencies)
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD node -e "fetch('http://localhost:' + (process.env.PORT || 3000) + '/api/v1/health').then(r => r.ok ? process.exit(0) : process.exit(1)).catch(() => process.exit(1))"

# Start ClauseLens Server
CMD ["node", "bin/server.js"]
