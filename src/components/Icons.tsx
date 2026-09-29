interface IconProps {
  size?: number;
  className?: string;
}

const base = (paths: React.ReactNode, size: number, className?: string) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.8}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
  >
    {paths}
  </svg>
);

export function IconSettings({ size = 16, className }: IconProps) {
  return base(
    <>
      <path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 0 0-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 0 0-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 0 0-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 0 0-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 0 0 1.066-2.573c-.94-1.543.826-3.31 2.37-2.37c1 .608 2.296.07 2.572-1.065z" />
      <path d="M9 12a3 3 0 1 0 6 0a3 3 0 0 0-6 0" />
    </>,
    size, className
  );
}

export function IconPencil({ size = 16, className }: IconProps) {
  return base(
    <>
      <path d="M4 20h4l10.5-10.5a2.828 2.828 0 1 0-4-4L4 16v4" />
      <path d="M13.5 6.5l4 4" />
    </>,
    size, className
  );
}

export function IconMessage({ size = 14, className }: IconProps) {
  return base(
    <>
      <path d="M8 9h8" />
      <path d="M8 13h6" />
      <path d="M18 4a3 3 0 0 1 3 3v8a3 3 0 0 1-3 3h-5l-5 3v-3h-2a3 3 0 0 1-3-3V7a3 3 0 0 1 3-3h12z" />
    </>,
    size, className
  );
}

export function IconCircleCheck({ size = 28, className }: IconProps) {
  return base(
    <>
      <path d="M12 12m-9 0a9 9 0 1 0 18 0a9 9 0 1 0-18 0" />
      <path d="M9 12l2 2l4-4" />
    </>,
    size, className
  );
}

export function IconRefresh({ size = 14, className }: IconProps) {
  return base(
    <>
      <path d="M20 11a8.1 8.1 0 0 0-15.5-2m-.5-4v4h4" />
      <path d="M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4" />
    </>,
    size, className
  );
}
