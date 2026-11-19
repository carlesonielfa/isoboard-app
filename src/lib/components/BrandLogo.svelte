<!-- Geometry: static/logo.svg. Each layer spins independently like a coin: clicking flips it
     around its vertical axis away from the clicked side, and repeated clicks add force. -->
<script lang="ts">
  import { onDestroy } from 'svelte';
  import { advance, flick, restingSpin, settled, type Spin } from '$lib/coin-spin';

  let { class: className = 'size-8 shrink-0' }: { class?: string } = $props();

  const layers = [
    'M 147.40804,21.472229 22.450839,64.517352 a 7.0107444,7.0107444 125.49612 0 0 -4.727373,6.62848 l 0,3.229167 c 0,2.761424 1.755582,3.594747 4.129523,2.189077 6.075782,-3.59762 18.955379,-9.469195 39.293968,-10.315381 2.758715,-0.114777 7.240958,-0.11116 9.992014,0.118549 32.672809,2.728136 90.572439,29.798317 122.305319,32.614947 2.74967,0.244062 7.23243,0.248472 9.99127,0.133688 20.33893,-0.846223 33.2187,-6.717913 39.29444,-10.315511 2.37384,-1.405606 4.12935,-5.049362 4.12935,-7.810786 v -9.84375 a 8.009429,8.009429 58.024942 0 0 -4.49206,-7.195767 L 156.62747,22.039516 a 12.053917,12.053917 3.5210663 0 0 -9.21943,-0.567287 z',
    'm 61.147251,99.321621 c -20.338794,0.846229 -33.21863,6.717939 -39.29442,10.315539 -2.373843,1.40559 -4.129365,5.04934 -4.129365,7.81076 v 23.07291 c 0,2.76142 1.75558,3.59475 4.12952,2.18907 6.075776,-3.59762 18.955363,-9.46919 39.293969,-10.31538 2.758716,-0.11477 7.24096,-0.11116 9.992016,0.11855 32.672829,2.72814 90.572419,29.79832 122.305319,32.61495 2.74967,0.24406 7.23243,0.24847 9.99127,0.13369 20.33895,-0.84622 33.2187,-6.71791 39.29444,-10.31551 2.37384,-1.40561 4.12935,-5.04937 4.12935,-7.81079 V 124.0625 c 0,-2.76142 -1.75558,-3.59475 -4.12952,-2.18908 -6.07579,3.59762 -18.95539,9.46919 -39.29397,10.31538 -2.75872,0.11477 -7.24101,0.11149 -9.99197,-0.11925 -32.62869,-2.73679 -89.6759,-29.89195 -122.304536,-32.628699 -2.750913,-0.230735 -7.233265,-0.234017 -9.992103,-0.11923 z',
    'm 61.147251,165.46745 c -20.338811,0.84623 -33.218638,6.71794 -39.294422,10.31553 -2.373842,1.4056 -4.129363,5.04935 -4.129363,7.81077 v 9.84375 a 8.0094092,8.0094092 58.024878 0 0 4.492065,7.19576 l 85.740309,41.91055 a 12.053936,12.053936 3.5209724 0 0 9.21944,0.56727 l 124.9567,-43.04509 a 7.0107524,7.0107524 125.49609 0 0 4.72737,-6.62849 v -3.22917 c 0,-2.76142 -1.75558,-3.59475 -4.12952,-2.18908 -6.07578,3.59763 -18.95537,9.4692 -39.29397,10.31539 -2.75872,0.11477 -7.24101,0.11149 -9.99197,-0.11925 -32.62871,-2.73679 -89.67588,-29.89196 -122.304536,-32.62871 -2.750913,-0.23073 -7.233265,-0.23401 -9.992103,-0.11923 z',
  ];

  let spins = $state.raw<Spin[]>(layers.map(() => restingSpin));
  let frame = 0;
  let last = 0;

  function tick(now: number): void {
    const seconds = Math.min((now - last) / 1000, 0.05);
    last = now;
    spins = spins.map((spin) => (settled(spin) ? restingSpin : advance(spin, seconds)));
    frame = spins.every((spin) => spin === restingSpin) ? 0 : requestAnimationFrame(tick);
  }

  // Clicks are matched against each layer's resting shape, so a layer stays easy to hit
  // while it is turned edge-on. Points this close to a layer still count as a hit.
  const VIEWBOX = 264.58333;
  const TOLERANCE = 18;
  const probes = [[0, 0]].concat(
    [0.5, 1].flatMap((reach) =>
      Array.from({ length: 8 }, (_, step) => [
        Math.cos((step * Math.PI) / 4) * TOLERANCE * reach,
        Math.sin((step * Math.PI) / 4) * TOLERANCE * reach,
      ]),
    ),
  );
  let shapes: SVGPathElement[] = [];

  function layerAt(x: number, y: number): number {
    for (const [dx, dy] of probes) {
      const point = new DOMPoint(x + dx, y + dy);
      const index = shapes.findIndex((shape) => shape.isPointInFill(point));
      if (index >= 0) return index;
    }
    return -1;
  }

  function handleClick(event: MouseEvent): void {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = (event.clientX - box.left) / box.width;
    const index = layerAt(x * VIEWBOX, ((event.clientY - box.top) / box.height) * VIEWBOX);
    if (index < 0) return;
    spins = spins.with(index, flick(spins[index], x * 2 - 1));
    if (!frame) {
      last = performance.now();
      frame = requestAnimationFrame(tick);
    }
  }

  onDestroy(() => cancelAnimationFrame(frame));
</script>

<span class={`relative block select-none ${className}`} aria-hidden="true" onclick={handleClick}>
  {#each layers as d, i (i)}
    <svg
      viewBox={`0 0 ${VIEWBOX} ${VIEWBOX}`}
      class="pointer-events-none absolute inset-0 size-full fill-current text-ink"
      style:transform={spins[i] === restingSpin
        ? undefined
        : `perspective(6em) rotateY(${spins[i].angle}rad)`}
    >
      <path {d} bind:this={shapes[i]} />
    </svg>
  {/each}
</span>
