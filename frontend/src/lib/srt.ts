export interface SRTEntry {
  index: number;
  startTime: string;
  endTime: string;
  text: string;
}

export function parseSRT(srtText: string): SRTEntry[] {
  const entries: SRTEntry[] = [];
  const trimmed = srtText.trim();
  if (!trimmed) {
    return entries;
  }

  const blocks = trimmed.split(/\n\s*\n/);

  for (const block of blocks) {
    const lines = block.trim().split('\n');
    if (lines.length < 3) {
      continue;
    }

    const index = Number.parseInt(lines[0] ?? '0', 10);
    const timeLine = lines[1] ?? '';
    const text = lines.slice(2).join('\n');
    const timeMatch = timeLine.match(/(\S+)\s*-->\s*(\S+)/);
    if (!timeMatch) {
      continue;
    }

    entries.push({
      index,
      startTime: timeMatch[1] ?? '',
      endTime: timeMatch[2] ?? '',
      text,
    });
  }

  return entries;
}

export function sliceSRT(srtText: string, start: number, end: number): string {
  const entries = parseSRT(srtText).filter((entry) => entry.index >= start && entry.index <= end);
  return entries.map((entry) => `${entry.index}\n${entry.startTime} --> ${entry.endTime}\n${entry.text}`).join('\n\n');
}

export function calculateSRTDuration(srtText: string): number {
  const entries = parseSRT(srtText);
  const firstEntry = entries[0];
  const lastEntry = entries[entries.length - 1];
  if (!firstEntry || !lastEntry) {
    return 0;
  }

  return timeToSeconds(lastEntry.endTime) - timeToSeconds(firstEntry.startTime);
}

function timeToSeconds(timeStr: string): number {
  const match = timeStr.match(/(\d+):(\d+):(\d+)[,.](\d+)/);
  if (!match) {
    return 0;
  }

  const hours = Number.parseInt(match[1] ?? '0', 10);
  const minutes = Number.parseInt(match[2] ?? '0', 10);
  const seconds = Number.parseInt(match[3] ?? '0', 10);
  const milliseconds = Number.parseInt(match[4] ?? '0', 10);

  return hours * 3600 + minutes * 60 + seconds + milliseconds / 1000;
}

export function isValidSRT(text: string): boolean {
  try {
    return parseSRT(text).length > 0;
  } catch {
    return false;
  }
}

export function extractTextFromSRT(srtText: string): string {
  return parseSRT(srtText)
    .map((entry) => entry.text)
    .join('\n');
}
