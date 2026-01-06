# =============================================================================
# PIRAMID DOCKERFILE
# Python FastAPI server + Next.js dashboard
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build the Next.js dashboard
# -----------------------------------------------------------------------------
FROM node:20-slim AS dashboard-builder

WORKDIR /app/dashboard

# Copy package files
COPY dashboard/package.json dashboard/package-lock.json ./

# Install dependencies
RUN npm ci

# Copy source files
COPY dashboard ./

# Build static export
RUN npm run build

# -----------------------------------------------------------------------------
# Stage 2: Python runtime
# -----------------------------------------------------------------------------
FROM python:3.12-slim

WORKDIR /app

# Install dependencies
COPY server/requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

# Copy server code
COPY server/*.py ./

# Copy dashboard from builder
COPY --from=dashboard-builder /app/dashboard/out ./dashboard

# Create data directory
RUN mkdir -p /app/data

# Environment variables
ENV PIRAMID_HOST=0.0.0.0
ENV PIRAMID_PORT=6333
ENV PIRAMID_DATA_DIR=/app/data
ENV PIRAMID_DASHBOARD_PATH=/app/dashboard

# Expose the port
EXPOSE 6333

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:6333/api/health || exit 1

# Run the server
CMD ["python", "main.py"]
