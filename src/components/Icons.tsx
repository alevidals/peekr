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
    aria-hidden="true"
    focusable="false"
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

export function IconChevron({ size = 14, className }: IconProps) {
  return base(<path d="M9 6l6 6l-6 6" />, size, className);
}

export function IconTrash({ size = 14, className }: IconProps) {
  return base(
    <>
      <path d="M4 7h16" />
      <path d="M10 11v6M14 11v6" />
      <path d="M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2l1-12" />
      <path d="M9 7V4h6v3" />
    </>,
    size, className
  );
}

export function IconPlus({ size = 14, className }: IconProps) {
  return base(<path d="M12 5v14M5 12h14" />, size, className);
}

export function IconCheck({ size = 14, className }: IconProps) {
  return base(<path d="M5 12l5 5L20 7" />, size, className);
}

export function IconAlert({ size = 14, className }: IconProps) {
  return base(
    <>
      <path d="M12 9v4M12 17h.01" />
      <path d="M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
    </>,
    size, className
  );
}

export function IconBranch({ size = 12, className }: IconProps) {
  return base(
    <>
      <circle cx="6" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <circle cx="18" cy="8" r="2" />
      <path d="M6 8v8M18 10c0 4-6 3-12 6" />
    </>,
    size, className
  );
}

export function IconGithub({ size = 16, className }: IconProps) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2c-3.2.7-3.87-1.54-3.87-1.54-.52-1.33-1.28-1.69-1.28-1.69-1.04-.71.08-.7.08-.7 1.15.08 1.76 1.18 1.76 1.18 1.03 1.76 2.69 1.25 3.35.96.1-.75.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.18-3.09-.12-.29-.51-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.62 1.59.23 2.76.11 3.05.74.8 1.18 1.83 1.18 3.09 0 4.42-2.7 5.4-5.26 5.68.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.79.56A11.5 11.5 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5z" />
    </svg>
  );
}

export function IconBitbucket({ size = 16, className }: IconProps) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M2.7 3.5a.7.7 0 0 0-.7.8l2.9 17.6c.1.4.4.7.8.7h13.8c.3 0 .6-.2.6-.6L23 4.3a.7.7 0 0 0-.7-.8H2.7zm12.5 12.3H9L7.7 9h8.7l-1.2 6.8z" />
    </svg>
  );
}

export function ProviderIcon({ kind, size = 16, className }: IconProps & { kind: "bitbucket" | "github" }) {
  return kind === "github" ? (
    <IconGithub size={size} className={className} />
  ) : (
    <IconBitbucket size={size} className={className} />
  );
}

export function IconSearch({ size = 16, className }: IconProps) {
  return base(
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-3.5-3.5" />
    </>,
    size, className
  );
}

export function IconX({ size = 14, className }: IconProps) {
  return base(<path d="M6 6l12 12M18 6L6 18" />, size, className);
}

export function IconSearchOff({ size = 28, className }: IconProps) {
  return base(
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-3.5-3.5M8.5 8.5l5 5M13.5 8.5l-5 5" />
    </>,
    size, className
  );
}
