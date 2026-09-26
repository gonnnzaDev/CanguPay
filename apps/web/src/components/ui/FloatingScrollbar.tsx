"use client";

import React, { useEffect, useState, useRef, useCallback } from "react";

const THUMB_HEIGHT = 42; // Compact, elegant floating pill height (px)
const EDGE_OFFSET = 12; // Gap from top and bottom viewport boundaries (px)
const LERP_FACTOR = 0.22; // Silky smooth deceleration coefficient for wheel/trackpad scroll

export function FloatingScrollbar() {
  const [isVisible, setIsVisible] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [isInteracting, setIsInteracting] = useState(false);

  const thumbRef = useRef<HTMLDivElement | null>(null);
  const trackRef = useRef<HTMLDivElement | null>(null);

  // Position coordinates kept in mutable refs for zero-overhead 120fps hardware acceleration
  const currentYRef = useRef(EDGE_OFFSET);
  const targetYRef = useRef(EDGE_OFFSET);
  const rafIdRef = useRef<number | null>(null);
  const idleTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const isDraggingRef = useRef(false);
  const startDragRef = useRef({ startY: 0, startScroll: 0 });

  // Compute target position from current scroll state
  const computeTargetY = useCallback(() => {
    const scrollHeight = document.documentElement.scrollHeight;
    const clientHeight = window.innerHeight;
    const maxScroll = scrollHeight - clientHeight;

    if (maxScroll <= 10) {
      return { visible: false, targetY: EDGE_OFFSET };
    }

    const scrollY = window.scrollY || document.documentElement.scrollTop;
    const progress = Math.min(Math.max(scrollY / maxScroll, 0), 1);
    const availableTrack = clientHeight - THUMB_HEIGHT - EDGE_OFFSET * 2;
    const targetY = EDGE_OFFSET + progress * Math.max(availableTrack, 0);

    return { visible: true, targetY };
  }, []);

  // Smooth LERP animation loop
  const startAnimation = useCallback(() => {
    if (rafIdRef.current !== null) return;

    const tick = () => {
      if (isDraggingRef.current) {
        rafIdRef.current = null;
        return;
      }

      const diff = targetYRef.current - currentYRef.current;

      if (Math.abs(diff) < 0.15) {
        currentYRef.current = targetYRef.current;
        if (thumbRef.current) {
          thumbRef.current.style.transform = `translate3d(0, ${currentYRef.current}px, 0)`;
        }
        rafIdRef.current = null;
        return;
      }

      currentYRef.current += diff * LERP_FACTOR;
      if (thumbRef.current) {
        thumbRef.current.style.transform = `translate3d(0, ${currentYRef.current}px, 0)`;
      }

      rafIdRef.current = requestAnimationFrame(tick);
    };

    rafIdRef.current = requestAnimationFrame(tick);
  }, []);

  // Update target on scroll or resize
  const onScrollOrResize = useCallback(() => {
    if (isDraggingRef.current) return;

    const { visible, targetY } = computeTargetY();
    setIsVisible((prev) => (prev !== visible ? visible : prev));

    if (!visible) return;

    targetYRef.current = targetY;
    setIsInteracting(true);

    if (idleTimeoutRef.current) clearTimeout(idleTimeoutRef.current);
    idleTimeoutRef.current = setTimeout(() => {
      setIsInteracting(false);
    }, 1200);

    startAnimation();
  }, [computeTargetY, startAnimation]);

  useEffect(() => {
    const handleScroll = () => {
      onScrollOrResize();
    };

    const handleResize = () => {
      onScrollOrResize();
    };

    const initialRaf = requestAnimationFrame(() => {
      const { visible, targetY } = computeTargetY();
      setIsVisible(visible);
      currentYRef.current = targetY;
      targetYRef.current = targetY;
      if (thumbRef.current) {
        thumbRef.current.style.transform = `translate3d(0, ${targetY}px, 0)`;
      }
    });

    window.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("resize", handleResize, { passive: true });

    const observer = new ResizeObserver(() => {
      requestAnimationFrame(() => {
        onScrollOrResize();
      });
    });
    observer.observe(document.body);

    return () => {
      cancelAnimationFrame(initialRaf);
      if (rafIdRef.current !== null) cancelAnimationFrame(rafIdRef.current);
      if (idleTimeoutRef.current) clearTimeout(idleTimeoutRef.current);
      window.removeEventListener("scroll", handleScroll);
      window.removeEventListener("resize", handleResize);
      observer.disconnect();
    };
  }, [computeTargetY, onScrollOrResize]);

  // Direct 1:1 Thumb Dragging Handler
  const handleThumbPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();

    if (rafIdRef.current !== null) {
      cancelAnimationFrame(rafIdRef.current);
      rafIdRef.current = null;
    }

    isDraggingRef.current = true;
    setIsDragging(true);
    setIsInteracting(true);

    const startY = e.clientY;
    const startScroll = window.scrollY || document.documentElement.scrollTop;
    startDragRef.current = { startY, startScroll };

    (e.target as HTMLElement).setPointerCapture(e.pointerId);

    const onPointerMove = (moveEvent: PointerEvent) => {
      if (!isDraggingRef.current) return;

      const scrollHeight = document.documentElement.scrollHeight;
      const clientHeight = window.innerHeight;
      const maxScroll = scrollHeight - clientHeight;
      const availableTrack = clientHeight - THUMB_HEIGHT - EDGE_OFFSET * 2;

      if (availableTrack <= 0 || maxScroll <= 0) return;

      const deltaY = moveEvent.clientY - startDragRef.current.startY;
      const scrollDelta = (deltaY / availableTrack) * maxScroll;
      const targetScroll = Math.min(
        Math.max(startDragRef.current.startScroll + scrollDelta, 0),
        maxScroll
      );

      // Instant 1:1 scroll without jitter
      window.scrollTo({ top: targetScroll, behavior: "instant" });

      const progress = targetScroll / maxScroll;
      const newY = EDGE_OFFSET + progress * availableTrack;
      currentYRef.current = newY;
      targetYRef.current = newY;

      if (thumbRef.current) {
        thumbRef.current.style.transform = `translate3d(0, ${newY}px, 0)`;
      }
    };

    const onPointerUp = (upEvent: PointerEvent) => {
      isDraggingRef.current = false;
      setIsDragging(false);

      if (idleTimeoutRef.current) clearTimeout(idleTimeoutRef.current);
      idleTimeoutRef.current = setTimeout(() => {
        setIsInteracting(false);
      }, 1200);

      try {
        (e.target as HTMLElement).releasePointerCapture(upEvent.pointerId);
      } catch {}

      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  };

  // Click on Track to smoothly glide to position
  const handleTrackClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === thumbRef.current) return;

    const clientHeight = window.innerHeight;
    const scrollHeight = document.documentElement.scrollHeight;
    const maxScroll = scrollHeight - clientHeight;
    const availableTrack = clientHeight - THUMB_HEIGHT - EDGE_OFFSET * 2;

    if (availableTrack <= 0 || maxScroll <= 0) return;

    const clickY = e.clientY;
    const desiredThumbTop = clickY - THUMB_HEIGHT / 2;
    const progress = Math.min(
      Math.max((desiredThumbTop - EDGE_OFFSET) / availableTrack, 0),
      1
    );
    const targetScroll = progress * maxScroll;

    window.scrollTo({ top: targetScroll, behavior: "smooth" });
  };

  if (!isVisible) return null;

  return (
    <div
      ref={trackRef}
      onClick={handleTrackClick}
      aria-hidden="true"
      className="fixed right-0 top-0 bottom-0 w-3.5 z-50 select-none cursor-pointer group"
    >
      <div
        ref={thumbRef}
        onPointerDown={handleThumbPointerDown}
        style={{
          height: `${THUMB_HEIGHT}px`,
          transform: `translate3d(0, ${EDGE_OFFSET}px, 0)`,
          willChange: "transform",
        }}
        className={`absolute right-1.5 rounded-full cursor-grab active:cursor-grabbing transition-[width,background-color,opacity,box-shadow] duration-200 ease-out ${
          isDragging
            ? "w-2 bg-[#0E2F46] dark:bg-[#0EAFB6] shadow-[0_0_12px_rgba(14,175,182,0.85)] opacity-100"
            : isInteracting
              ? "w-1.5 group-hover:w-2 bg-[#0E2F46]/60 dark:bg-white/40 dark:group-hover:bg-[#0EAFB6] group-hover:shadow-[0_0_8px_rgba(14,175,182,0.6)] opacity-90"
              : "w-1 group-hover:w-2 bg-[#0E2F46]/30 group-hover:bg-[#0E2F46]/60 dark:bg-white/25 dark:group-hover:bg-[#0EAFB6] group-hover:shadow-[0_0_8px_rgba(14,175,182,0.6)] opacity-40 group-hover:opacity-100"
        }`}
      />
    </div>
  );
}
