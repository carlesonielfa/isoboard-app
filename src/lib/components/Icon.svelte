<script lang="ts">
  type IconName = 'plus' | 'pencil' | 'trash' | 'close' | 'chevron';

  let {
    name,
    class: className = '',
  }: {
    name: IconName;
    class?: string;
  } = $props();

  // The existing action outlines are adapted from HackerNoon's Pixel Icon
  // Library and are kept inline so the app carries only the glyphs it uses.
  const paths: Record<IconName, string> = {
    plus: 'M23 13L23 11L13 11L13 1L11 1L11 11L1 11L1 13L11 13L11 23L13 23L13 13L23 13z',
    pencil:
      'M22 20V21H21V22H20V23H16V22H15V21H14V20H13V19H12V18H11V17H10V16H9V15H8V14H7V13H6V12H5V11H4V10H3V9H2V8H1V1H8V2H9V3H10V4H11V5H12V6H13V7H14V8H15V9H16V10H17V11H18V12H19V13H20V14H21V15H22V16H23V20H22zM8 4H7V3H4V4H3V7H4V8H5V7H6V6H7V5H8V4zM17 13H16V12H15V11H14V10H13V9H12V8H11V7H10V6H9V7H8V8H7V9H6V10H7V11H8V12H9V13H10V14H11V15H12V16H13V17H15V16H16V15H17V13zM18 16V17H17V18H16V20H17V21H19V20H20V19H21V17H20V16H18z',
    trash:
      'M4 18V10H5V2H6V1H18V2H19V10H20V18H4zM18 11H17V3H7V11H6V16H18V11zM21 21L21 19L3 19L3 21L4 21L4 22L9 22L9 23L15 23L15 22L20 22L20 21L21 21z',
    close:
      'M3 1H7V5H11V9H13V5H17V1H21V5H17V9H15V11H17V15H21V19H17V15H13V13H11V15H7V19H3V15H7V11H9V9H7V5H3V1z',
    // This compact disclosure glyph is app-owned.
    chevron:
      'M3 17H7V15H9V13H11V11H13V13H15V15H17V17H21V15H19V13H17V11H15V9H13V7H11V9H9V11H7V13H5V15H3V17z',
  };
</script>

<!--
  HackerNoon Pixel Icon Library (plus, pencil, trash), CC BY 4.0:
  https://github.com/hackernoon/pixel-icon-library
  https://creativecommons.org/licenses/by/4.0/
  Glyphs are adapted from the package's 24px iconfont.svg into inline paths.
-->
<svg
  class={`size-4 shrink-0 ${className}`}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
  focusable="false"
>
  <path d={paths[name]} fill="currentColor" transform="matrix(1 0 0 -1 0 24)"></path>
</svg>
