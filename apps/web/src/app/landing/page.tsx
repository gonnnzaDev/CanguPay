import type { Metadata } from "next";
import { LandingHeader } from "@/components/landing/LandingHeader";
import { LandingHero } from "@/components/landing/LandingHero";
import { LandingFlow } from "@/components/landing/LandingFlow";
import { LandingTriad } from "@/components/landing/LandingTriad";
import { LandingProductPreview } from "@/components/landing/LandingProductPreview";
import { LandingStellar } from "@/components/landing/LandingStellar";
import { LandingCTA } from "@/components/landing/LandingCTA";

export const metadata: Metadata = {
  title: "CanguPay // B2B Conditional Escrow on Stellar",
  description:
    "Conditional B2B settlement platform on Stellar Soroban. The agent informs; the contract executes; the human resolves disputes.",
};

export default function LandingPage() {
  return (
    <div className="min-h-screen flex flex-col bg-transparent selection:bg-teal-500/20 selection:text-teal-700 dark:selection:text-teal-300">
      <LandingHeader />
      <main className="flex-1">
        <LandingHero />
        <LandingFlow />
        <LandingTriad />
        <LandingProductPreview />
        <LandingStellar />
      </main>
      <LandingCTA />
    </div>
  );
}
