export const menuSurfaceClasses =
  'relative z-50 rounded-control bg-popover/80 text-ink shadow-md ring-1 ring-ink/10 backdrop-blur-xl';

export const menuItemClasses =
  'cursor-default rounded-control text-sm text-ink outline-none data-[highlighted]:bg-surface-raised data-[disabled]:opacity-45';

export const selectionTriggerClasses =
  "border-border data-placeholder:text-ink focus-visible:border-focus focus-visible:ring-focus/30 aria-invalid:ring-danger/30 aria-invalid:border-danger-border gap-1.5 rounded-control border bg-surface py-2 pr-2 pl-3 text-sm font-normal text-ink transition-[background-color,border-color,box-shadow] duration-150 ease-in-out select-none focus-visible:ring-2 aria-invalid:ring-2 data-[size=default]:min-h-10 data-[size=sm]:min-h-9 data-[size=sm]:rounded-control *:data-[slot=select-value]:flex *:data-[slot=select-value]:min-w-0 *:data-[slot=select-value]:max-w-full *:data-[slot=select-value]:truncate *:data-[slot=select-value]:gap-1.5 [&_svg:not([class*='size-'])]:size-4 flex w-full min-w-0 max-w-full items-center justify-between overflow-hidden whitespace-nowrap outline-none disabled:cursor-not-allowed disabled:opacity-45 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center [&_svg]:pointer-events-none [&_svg]:shrink-0";
