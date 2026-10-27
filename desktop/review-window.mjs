// Electron uses DIP coordinates, so workArea can be passed straight to BrowserWindow.
export function reviewWindowBounds(displays, primaryId, width, height) {
  const secondary = displays.find(display => display.id !== primaryId && display.workArea?.width >= 960 && display.workArea?.height >= 720);
  if (!secondary) return null;
  const area = secondary.workArea;
  const actualWidth = Math.min(width, area.width);
  const actualHeight = Math.min(height, area.height);
  return {
    x: area.x + Math.floor((area.width - actualWidth) / 2),
    y: area.y + Math.floor((area.height - actualHeight) / 2),
    width: actualWidth,
    height: actualHeight,
  };
}
