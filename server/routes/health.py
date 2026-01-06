"""
Health Check Routes

Simple endpoint to verify the server is running.
Used by the dashboard to check connectivity.
"""

from fastapi import APIRouter

router = APIRouter(tags=["health"])


@router.get("/health")
async def health_check():
    """
    Check if the server is running.
    
    Returns basic server info for monitoring and debugging.
    The dashboard calls this endpoint to show connection status.
    """
    return {
        "status": "ok",
        "service": "piramid",
        "version": "0.1.0",
    }
