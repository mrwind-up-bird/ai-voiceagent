'use client';

import { useCallback } from 'react';

type ResizeDirection = 'bottom' | 'bottomLeft' | 'bottomRight';

export function ResizeHandle() {
  const startResize = useCallback(async (direction: ResizeDirection) => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const window = getCurrentWindow();

      // Map direction to Tauri resize direction
      const directionMap: Record<ResizeDirection, string> = {
        bottom: 'South',
        bottomLeft: 'SouthWest',
        bottomRight: 'SouthEast',
      };

      await window.startResizeDragging(directionMap[direction] as any);
    } catch (err) {
      // Running in browser, ignore
      console.log('Resize not available:', err);
    }
  }, []);

  return (
    <>
      {/* Bottom edge - full width resize handle */}
      <div
        className="absolute bottom-0 left-8 right-8 h-2 cursor-s-resize group"
        onMouseDown={() => startResize('bottom')}
      >
        <div className="absolute bottom-0.5 left-1/2 -translate-x-1/2 w-12 h-1 rounded-full bg-gray-600 opacity-0 group-hover:opacity-100 transition-opacity" />
      </div>

      {/* Bottom-left corner */}
      <div
        className="absolute bottom-0 left-0 w-4 h-4 cursor-sw-resize"
        onMouseDown={() => startResize('bottomLeft')}
      />

      {/* Bottom-right corner */}
      <div
        className="absolute bottom-0 right-0 w-4 h-4 cursor-se-resize"
        onMouseDown={() => startResize('bottomRight')}
      />
    </>
  );
}
