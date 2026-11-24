// Short synthesized chimes (no audio files, keeps the app small).

let ctx: AudioContext | null = null;

function tone(freq: number, start: number, dur: number, gain = 0.12) {
  if (!ctx) return;
  const o = ctx.createOscillator();
  const g = ctx.createGain();
  o.type = "sine";
  o.frequency.value = freq;
  const t = ctx.currentTime + start;
  g.gain.setValueAtTime(0, t);
  g.gain.linearRampToValueAtTime(gain, t + 0.015);
  g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  o.connect(g).connect(ctx.destination);
  o.start(t);
  o.stop(t + dur + 0.05);
}

/** "done": rising two notes · "attention": two quick high notes · "error": falling */
export function chime(kind: "done" | "attention" | "error") {
  try {
    ctx ??= new AudioContext();
    if (ctx.state === "suspended") void ctx.resume();
    if (kind === "done") {
      tone(659, 0, 0.25);
      tone(988, 0.12, 0.35);
    } else if (kind === "attention") {
      tone(880, 0, 0.15);
      tone(880, 0.2, 0.2);
    } else {
      tone(523, 0, 0.25);
      tone(392, 0.15, 0.4);
    }
  } catch {
    // audio unavailable: stay silent
  }
}
