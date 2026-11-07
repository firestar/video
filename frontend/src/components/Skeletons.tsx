export function VideoCardSkeleton() {
  return (
    <div className="flex flex-col gap-3">
      <div className="skeleton aspect-video w-full" />
      <div className="flex gap-3">
        <div className="skeleton size-9 shrink-0 rounded-full" />
        <div className="flex-1 space-y-2">
          <div className="skeleton h-4 w-full" />
          <div className="skeleton h-3 w-2/3" />
          <div className="skeleton h-3 w-1/3" />
        </div>
      </div>
    </div>
  );
}

export function VideoGridSkeleton({ count = 10 }: { count?: number }) {
  return (
    <div className="grid grid-cols-1 gap-x-4 gap-y-8 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {Array.from({ length: count }).map((_, i) => (
        <VideoCardSkeleton key={i} />
      ))}
    </div>
  );
}

export function ListRowSkeleton() {
  return (
    <div className="flex items-center gap-4 p-2">
      <div className="skeleton h-14 w-24 shrink-0" />
      <div className="flex-1 space-y-2">
        <div className="skeleton h-4 w-3/4" />
        <div className="skeleton h-3 w-1/2" />
        <div className="skeleton h-3 w-1/3" />
      </div>
    </div>
  );
}

export function ListSkeleton({ count = 6 }: { count?: number }) {
  return (
    <div className="flex flex-col gap-2">
      {Array.from({ length: count }).map((_, i) => (
        <ListRowSkeleton key={i} />
      ))}
    </div>
  );
}

export function CommentSkeleton() {
  return (
    <div className="flex gap-3">
      <div className="skeleton size-8 shrink-0 rounded-full" />
      <div className="flex-1 space-y-2">
        <div className="skeleton h-3 w-1/3" />
        <div className="skeleton h-4 w-full" />
        <div className="skeleton h-3 w-1/4" />
      </div>
    </div>
  );
}

export function CommentSectionSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className="flex flex-col gap-5">
      {Array.from({ length: count }).map((_, i) => (
        <CommentSkeleton key={i} />
      ))}
    </div>
  );
}

export function WatchPageSkeleton() {
  return (
    <div className="mx-auto w-full max-w-[1750px] px-4 py-4 lg:px-8">
      <div className="flex flex-col gap-6 lg:flex-row lg:gap-8">
        <div className="min-w-0 flex-1">
          <div className="skeleton aspect-video w-full" />
          <div className="mt-4 skeleton h-6 w-3/4" />
          <div className="mt-4 flex items-center gap-3">
            <div className="skeleton size-10 shrink-0 rounded-full" />
            <div className="space-y-2">
              <div className="skeleton h-4 w-40" />
              <div className="skeleton h-3 w-24" />
            </div>
          </div>
          <div className="mt-4 skeleton h-24 w-full" />
        </div>
        <aside className="hidden w-[400px] shrink-0 lg:block xl:w-[440px]">
          <div className="space-y-4">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="flex gap-3">
                <div className="skeleton h-16 w-28 shrink-0" />
                <div className="flex-1 space-y-2">
                  <div className="skeleton h-3 w-full" />
                  <div className="skeleton h-3 w-2/3" />
                </div>
              </div>
            ))}
          </div>
        </aside>
      </div>
    </div>
  );
}

export function ChannelPageSkeleton() {
  return (
    <div>
      <div className="skeleton h-[200px] w-full rounded-none" />
      <div className="mx-auto max-w-6xl px-4">
        <div className="flex flex-col gap-6 pt-4 sm:flex-row sm:items-end">
          <div className="skeleton size-32 shrink-0 rounded-full" />
          <div className="flex-1 space-y-2">
            <div className="skeleton h-7 w-64" />
            <div className="skeleton h-4 w-40" />
            <div className="skeleton h-3 w-32" />
          </div>
        </div>
        <div className="mt-10 space-y-4">
          <div className="skeleton h-5 w-24" />
          <div className="grid grid-cols-1 gap-x-4 gap-y-8 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
            {Array.from({ length: 8 }).map((_, i) => (
              <VideoCardSkeleton key={i} />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
