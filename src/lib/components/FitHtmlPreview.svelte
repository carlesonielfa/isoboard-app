<script lang="ts">
  let {
    html,
    title,
    class: className = '',
  }: {
    html: string;
    title: string;
    class?: string;
  } = $props();

  let frame: HTMLIFrameElement;
  let scheduledFrame: number | null = null;

  function numericStyle(value: string): number {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function applyDefaultStyles(document: Document): void {
    if (document.head.querySelector('[data-isoboard-preview-defaults]')) return;
    const style = document.createElement('style');
    style.dataset.isoboardPreviewDefaults = '';
    style.textContent = `
      :where(html) { font-family: sans-serif; }
      :where(html, body) { background: transparent; }
      :where(table) { width: 100%; }
      :where(img) {
        display: block;
        max-width: 100% !important;
        max-height: 112px !important;
        width: auto !important;
        height: auto !important;
        object-fit: contain;
      }
    `;
    document.head.appendChild(style);
  }

  function fitContent(): void {
    const document = frame.contentDocument;
    const body = document?.body;
    const root = document?.documentElement;
    if (!body || !root) return;
    applyDefaultStyles(document);

    let content = body.querySelector<HTMLElement>(':scope > [data-isoboard-fit-preview]');
    if (!content) {
      content = document.createElement('div');
      content.dataset.isoboardFitPreview = '';
      while (body.firstChild) content.appendChild(body.firstChild);
      body.appendChild(content);
    }
    const fittedContent = content;

    fittedContent.style.transform = 'none';
    fittedContent.style.transformOrigin = 'top left';
    fittedContent.style.display = 'flow-root';
    fittedContent.style.width = 'auto';
    root.style.overflow = 'hidden';
    body.style.overflow = 'hidden';

    const bodyStyle = frame.contentWindow?.getComputedStyle(body);
    const horizontalInset = bodyStyle
      ? numericStyle(bodyStyle.marginLeft) +
        numericStyle(bodyStyle.marginRight) +
        numericStyle(bodyStyle.paddingLeft) +
        numericStyle(bodyStyle.paddingRight)
      : 0;
    const verticalInset = bodyStyle
      ? numericStyle(bodyStyle.marginTop) +
        numericStyle(bodyStyle.marginBottom) +
        numericStyle(bodyStyle.paddingTop) +
        numericStyle(bodyStyle.paddingBottom)
      : 0;
    const availableWidth = Math.max(1, frame.clientWidth - horizontalInset - 1);
    const availableHeight = Math.max(1, frame.clientHeight - verticalInset - 1);

    function measureAtScale(scale: number): {
      width: number;
      height: number;
    } {
      fittedContent.style.width = `${availableWidth / scale}px`;
      return {
        width: Math.max(1, fittedContent.scrollWidth, fittedContent.offsetWidth),
        height: Math.max(1, fittedContent.scrollHeight, fittedContent.offsetHeight),
      };
    }

    const initial = measureAtScale(1);
    let fittedScale = Math.min(1, availableWidth / initial.width, availableHeight / initial.height);

    if (fittedScale < 1) {
      let lowerScale = fittedScale;
      let upperScale = 1;

      for (let iteration = 0; iteration < 12; iteration += 1) {
        const candidateScale = (lowerScale + upperScale) / 2;
        const candidate = measureAtScale(candidateScale);
        const fits =
          candidate.width * candidateScale <= availableWidth + 0.5 &&
          candidate.height * candidateScale <= availableHeight + 0.5;

        if (fits) lowerScale = candidateScale;
        else upperScale = candidateScale;
      }

      fittedScale = lowerScale;
    }

    measureAtScale(fittedScale);

    fittedContent.style.transform = `scale(${fittedScale})`;
  }

  function scheduleFit(): void {
    if (scheduledFrame !== null) cancelAnimationFrame(scheduledFrame);
    scheduledFrame = requestAnimationFrame(() => {
      scheduledFrame = null;
      fitContent();
    });
  }

  function handleLoad(): void {
    scheduleFit();
    void frame.contentDocument?.fonts?.ready.then(scheduleFit);
  }

  $effect(() => {
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(frame);
    return () => {
      observer.disconnect();
      if (scheduledFrame !== null) cancelAnimationFrame(scheduledFrame);
      scheduledFrame = null;
    };
  });
</script>

<iframe
  bind:this={frame}
  class={`h-full min-h-0 w-full overflow-hidden bg-surface-preview ${className}`}
  {title}
  srcdoc={html}
  sandbox="allow-same-origin"
  scrolling="no"
  onload={handleLoad}
></iframe>
