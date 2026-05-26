interface WallGridSkeletonProps {
  count?: number;
}

export function WallGridSkeleton({ count = 6 }: WallGridSkeletonProps) {
  return (
    <>
      {Array.from({ length: count }, (_, index) => (
        <article
          aria-hidden="true"
          className="wall-card wall-skeleton"
          key={index}
        >
          <div className="wall-thumb skeleton-block" />
          <div className="skeleton-meta">
            <span className="skeleton-line wide" />
            <span className="skeleton-line" />
          </div>
        </article>
      ))}
    </>
  );
}
