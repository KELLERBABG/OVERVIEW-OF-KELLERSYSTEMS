FROM node:20-alpine AS builder
WORKDIR /app
COPY . .
RUN corepack enable && pnpm install && pnpm build

FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app/packages/signaling-server/dist ./dist
COPY --from=builder /app/packages/signaling-server/package.json .
EXPOSE 8765
CMD ["node", "dist/index.js"]