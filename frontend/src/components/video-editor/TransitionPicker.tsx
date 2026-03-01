import type { ClipTransition } from './types';

interface TransitionPickerProps {
  value: ClipTransition;
  onChange: (next: ClipTransition) => void;
}

export function TransitionPicker({ value, onChange }: TransitionPickerProps) {
  return (
    <select
      className="glass-select-base h-8 min-w-28 px-2 text-xs"
      value={value}
      onChange={(event) => onChange(event.target.value as ClipTransition)}
    >
      <option value="none">None</option>
      <option value="fade">Fade</option>
      <option value="slide-left">Slide Left</option>
      <option value="zoom-in">Zoom In</option>
    </select>
  );
}
