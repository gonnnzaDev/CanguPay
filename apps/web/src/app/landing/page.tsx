import type { Metadata } from "next";
import { LandingHeader } from "@/components/landing/LandingHeader";
import { LandingHero } from "@/components/landing/LandingHero";

export const metadata: Metadata = {
  title: "CanguPay // B2B Conditional Escrow on Stellar",
  description:
    "Conditional B2B settlement platform on Stellar Soroban. The agent informs; the contract executes; the human resolves disputes.",
};

export default function LandingPage() {
  return (
    <div className="min-h-screen flex flex-col">
      <LandingHeader />
      <main className="flex-1">
        <LandingHero />
        {/* Anchor targets for planned visual zones */}
        <div id="flow" />
        <div id="architecture" />
        <div id="simulation" />
      </main>
    </div>
  );
}
