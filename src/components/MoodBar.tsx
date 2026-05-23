import { Mood } from "../types";

const moods: Mood[] = ["dark", "nature", "city", "minimal", "coding", "calm", "anime"];

interface MoodBarProps {
  activeMood: Mood;
  onMoodSelect: (mood: Mood) => void;
}

export function MoodBar({ activeMood, onMoodSelect }: MoodBarProps) {
  return (
    <div className="mood-bar" aria-label="Wallpaper moods">
      {moods.map((mood) => (
        <button
          className={activeMood === mood ? "mood-chip active" : "mood-chip"}
          key={mood}
          onClick={() => onMoodSelect(mood)}
          type="button"
        >
          {mood}
        </button>
      ))}
    </div>
  );
}
