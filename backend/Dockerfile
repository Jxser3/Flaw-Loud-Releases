FROM node:20-bookworm-slim AS frontend-builder

WORKDIR /app/backend/frontend
COPY frontend/package*.json ./
RUN npm ci --include=dev
COPY frontend/ ./
RUN npm run build

FROM node:20-bookworm-slim AS runtime

ENV NODE_ENV=production
WORKDIR /app/backend

COPY package*.json ./
RUN npm ci --omit=dev && npm cache clean --force
COPY src/ ./src/
COPY --from=frontend-builder /app/dist /app/dist

USER node
EXPOSE 8080
CMD ["npm", "start"]
