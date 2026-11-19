// Plays the capture clip, then the paste clip, and loops. Each clip keeps its own
// aspect ratio, so the two are stacked and cross-faded instead of joined into one video.
for (const demo of document.querySelectorAll('[data-hero-demo]')) {
  const clips = [...demo.querySelectorAll('video')];
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (reducedMotion || clips.length === 0) continue;

  let current = 0;
  const show = (index) => {
    clips.forEach((clip, i) => clip.classList.toggle('is-active', i === index));
    const clip = clips[index];
    clip.currentTime = 0;
    clip.play().catch(() => {});
  };

  clips.forEach((clip, i) =>
    clip.addEventListener('ended', () => {
      current = (i + 1) % clips.length;
      show(current);
    }),
  );

  // Start only when visible, and pause while scrolled away.
  new IntersectionObserver(([entry]) => {
    if (entry.isIntersecting) clips[current].play().catch(() => {});
    else clips[current].pause();
  }).observe(demo);

  show(0);
}
