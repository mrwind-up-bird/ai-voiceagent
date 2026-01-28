'use client';

import { useEffect, useCallback } from 'react';

/**
 * Handles window resize for frameless windows using Tauri's resize API.
 * Renders a resize handle in the bottom-right corner.
 */
export function WindowControls() {
  const handleResize = useCallback(async (e: React.MouseEvent) => {
    e.preventDefault();
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const window = getCurrentWindow();
      // Start resizing from south-east (bottom-right) corner
      await window.startResizeDragging('SouthEast');
    } catch {
      // Running in browser, ignore
    }
  }, []);

  return (
    <div
      className="resize-handle"
      onMouseDown={handleResize}
      title="Resize window"
    />
  );
}

/**
 * Starts window drag operation when called.
 * Use on elements that should drag the window.
 */
export async function startWindowDrag() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const window = getCurrentWindow();
    await window.startDragging();
  } catch {
    // Running in browser, ignore
  }
}
