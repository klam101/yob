"use client";

import { ArrowLeft } from "lucide-react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";

/**
 * Returns to the feed with its filters intact: goes back in history when we
 * came from the app, otherwise opens the feed fresh (e.g. a pasted link).
 */
export function BackButton() {
  const router = useRouter();

  function goBack() {
    const fromApp = document.referrer && new URL(document.referrer).origin === window.location.origin;
    if (fromApp && window.history.length > 1) router.back();
    else router.push("/jobs");
  }

  return (
    <Button variant="ghost" size="sm" shape="pill" onClick={goBack}>
      <ArrowLeft /> Back to jobs
    </Button>
  );
}
