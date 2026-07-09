FROM node:20-alpine AS deps
RUN apk add --no-cache libc6-compat
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
COPY packages/widget ../packages/widget
RUN npm ci --ignore-scripts

FROM node:20-alpine AS builder
WORKDIR /app/frontend
COPY --from=deps /app/frontend/node_modules ./node_modules
COPY --from=deps /app/packages ../packages
COPY frontend .
ENV NEXT_TELEMETRY_DISABLED=1
RUN npm run build

FROM node:20-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 nextjs

COPY --from=builder --chown=nextjs:nodejs /app/frontend/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/frontend/.next/static ./frontend/.next/static

USER nextjs

EXPOSE 3000
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:3000/ || exit 1

CMD ["node", "frontend/server.js"]
