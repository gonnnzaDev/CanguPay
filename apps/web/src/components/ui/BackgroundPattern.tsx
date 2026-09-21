import React from "react";

/**
 * Custom Diagonal Technical & Domain Icon Pattern for CanguPay.
 * 1. Icons are the HERO of the pattern: bolder stroke (1.85px) and higher contrast.
 * 2. Underlying engineering grid is whisper-quiet (hairline 0.5px, ultra-low opacity).
 * 3. 45-degree diagonal layout (diamond matrix) flowing across the screen.
 * 4. Icons counter-rotated by +45 degrees so they sit perfectly upright.
 */
export function BackgroundPattern() {
  return (
    <div
      aria-hidden="true"
      className="fixed inset-0 pointer-events-none z-0 overflow-hidden select-none"
    >
      {/* 1. Atmospheric Ambient Brand Glow */}
      <div className="absolute -top-40 left-1/2 -translate-x-1/2 w-[900px] sm:w-[1300px] h-[550px] bg-gradient-to-b from-teal-500/15 via-teal-500/5 to-transparent dark:from-teal-400/16 dark:via-teal-500/6 dark:to-transparent rounded-full blur-3xl opacity-75 dark:opacity-85" />

      {/* 2. Diagonal Grid + Prominent Domain Icon Wallpaper */}
      <svg
        className="absolute inset-0 w-full h-full text-neutral-950/[0.13] dark:text-white/[0.12] stroke-neutral-950/[0.13] dark:stroke-white/[0.12] [mask-image:radial-gradient(ellipse_90%_80%_at_50%_15%,#000_50%,transparent_95%)]"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          {/* --- ICON DEFINITIONS (22x22 normalized with confident stroke) --- */}

          {/* 1. Official CanguPay Kangaroo Glyph */}
          <g id="pat-kangaroo">
            <g transform="scale(0.038) translate(-170, -145)">
              <path fill="currentColor" fillRule="evenodd" d="M647.608 230.105C641.968 183.116 621.518 169.087 580.322 151.426C574.428 172.721 572.825 194.305 584.082 214.292C593.587 231.172 608.231 238.067 624.979 246.049C611.175 284.099 594.365 316.35 555.199 333.862C509.392 354.344 476.827 342.785 433.318 376.514C366.506 428.305 371.63 524.751 279.487 548.975C255.468 555.048 230.183 553.803 206.877 545.398C196.457 541.605 185.345 536.185 175.267 531.456C209.727 598.792 288.714 604.19 350.106 572.718C360.161 567.486 369.878 561.63 379.201 555.183C380.963 553.972 385.12 550.766 390.447 546.657C406.485 534.287 433.124 513.741 436.918 514.869C440.03 515.795 442.272 518.639 443.734 521.419C448.604 530.688 445.402 544.89 442.284 554.248C439.205 563.489 429.748 578.727 419.994 594.444C406.549 616.109 392.538 638.684 393.889 647.717L395.293 649.715C407.095 649.816 418.917 649.707 430.745 649.598C452.904 649.393 475.08 649.189 497.164 650.365C518.353 651.493 537.951 658.464 558.343 663.576C565.755 665.432 594.194 672.427 594.705 658.787C594.711 652.535 584.824 640.176 579.967 636.093C542.796 604.851 491.714 614.46 448.225 622.668C456.417 609.705 466.372 597.871 476.266 586.107C503.644 553.559 530.566 521.552 518.46 467.685C513.517 445.692 503.076 426.371 492.689 406.471C514.905 421.717 544.04 443.355 567.864 454.496C547.531 422.17 567.036 395.812 601.479 413.45C615.542 398.509 626.139 386.619 634.761 367.68C638.196 360.135 642.727 334.39 646.776 331.612C657.566 324.209 672.942 325.025 688.474 325.849C705.674 326.762 723.065 327.685 734.629 317.466C741.3 311.569 744.442 297.755 735.9 291.955C697.743 267.75 698.792 236.071 647.608 230.105Z M677.633 262.894C674.03 260.519 669.424 260.281 665.595 262.271C659.854 265.251 657.548 272.272 660.402 278.077C663.254 283.88 670.223 286.343 676.091 283.622C680.006 281.807 682.631 278.014 682.951 273.712C683.271 269.408 681.237 265.269 677.633 262.894Z" />
              <path fill="currentColor" opacity="0.85" d="M691.739 422.673L690.528 421.187C669.587 466.891 631.15 504.075 579.001 478.851C556.873 468.146 539.148 454.973 520.223 439.417C541.212 483.633 543.466 511.977 520.805 556.003C514.357 567.397 504.605 580.971 495.519 590.482C557.807 581.323 630.147 577.544 670.719 522.678C693.127 492.376 697.046 458.79 691.739 422.673Z" />
            </g>
          </g>

          {/* 2. Escrow Custody Shield */}
          <g id="pat-shield-lock" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <path d="M11 20s7-3.5 7-9V5l-7-2.5L4 5v6c0 5.5 7 9 7 9" />
            <rect x="8" y="10" width="6" height="5" rx="1" />
            <path d="M9.5 10V8a1.5 1.5 0 0 1 3 0v2" />
          </g>

          {/* 3. CPUSD Currency & SAC Token */}
          <g id="pat-coins" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="8" cy="8" r="6" />
            <path d="M17 12a6 6 0 1 0-6 6" />
            <path d="M8 5v6" />
            <path d="M6 7h4" />
          </g>

          {/* 4. Key & Freighter Wallet Signature */}
          <g id="pat-key" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="7.5" cy="14.5" r="5" />
            <path d="m20 2-9 9" />
            <path d="m14.5 7.5 2.5 2.5" />
          </g>

          {/* 5. Dispute & Arbitration Scale */}
          <g id="pat-scale" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <path d="m15 15 3-7 3 7c-.8.6-1.8 1-3 1s-2.2-.4-3-1Z" />
            <path d="m1 15 3-7 3 7c-.8.6-1.8 1-3 1s-2.2-.4-3-1Z" />
            <path d="M6 20h10" />
            <path d="M11 2v18" />
            <path d="M2 6h2c2 0 5-1 7-2 2 1 5 2 7 2h2" />
          </g>

          {/* 6. Atomic Settlement Swap */}
          <g id="pat-swap" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <path d="m15 3 4 4-4 4" />
            <path d="M19 7H3" />
            <path d="m7 19-4-4 4-4" />
            <path d="M3 15h16" />
          </g>

          {/* 7. Timelock & Deadlines Clock */}
          <g id="pat-clock" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="9" />
            <polyline points="11 6 11 11 15 13" />
          </g>

          {/* 8. Buyer Party */}
          <g id="pat-user-buyer" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <path d="M18 19v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
            <circle cx="11" cy="7" r="4" />
          </g>

          {/* 9. Evidence Bundle Hash */}
          <g id="pat-hash" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <line x1="4" y1="8" x2="18" y2="8" />
            <line x1="4" y1="14" x2="18" y2="14" />
            <line x1="9" y1="3" x2="7" y2="19" />
            <line x1="15" y1="3" x2="13" y2="19" />
          </g>

          {/* 10. Soroban Ledger Layers */}
          <g id="pat-layers" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <polygon points="11 2 2 6.5 11 11 20 6.5 11 2" />
            <polyline points="2 11 11 15.5 20 11" />
            <polyline points="2 15.5 11 20 20 15.5" />
          </g>

          {/* 11. Attestation Document POD */}
          <g id="pat-file-check" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <path d="M13 2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V7z" />
            <polyline points="13 2 13 7 18 7" />
            <path d="m8 13.5 2 2 4-4" />
          </g>

          {/* 12. Supplier Warehouse / Corporate Entity */}
          <g id="pat-building" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <rect x="4" y="2" width="14" height="18" rx="2" ry="2" />
            <path d="M9 20v-3h4v3" />
            <path d="M8 6h.01M14 6h.01M8 10h.01M14 10h.01" />
          </g>

          {/* 13. Attestation Engine CLI / Bot */}
          <g id="pat-terminal" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="4 15 9 10 4 5" />
            <line x1="11" y1="17" x2="18" y2="17" />
          </g>

          {/* 14. Contract State Database */}
          <g id="pat-database" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <ellipse cx="11" cy="5" rx="8" ry="3" />
            <path d="M19 11c0 1.66-3.58 3-8 3s-8-1.34-8-3" />
            <path d="M3 5v12c0 1.66 3.58 3 8 3s8-1.34 8-3V5" />
          </g>

          {/* 15. Fast Ledger Finality Bolt */}
          <g id="pat-zap" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <polygon points="12 2 3 13 11 13 10 20 19 9 11 9 12 2" />
          </g>

          {/* 16. Identity Seal / Fingerprint */}
          <g id="pat-fingerprint" fill="none" stroke="currentColor" strokeWidth="1.85" strokeLinecap="round" strokeLinejoin="round">
            <path d="M11 9a2 2 0 0 0-2 2c0 1-.1 2.5-.2 4" />
            <path d="M13 12c0 2.4 0 5-1 7" />
            <path d="M2 11h1a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2H2" />
            <path d="M16 11a5 5 0 0 0-10 0" />
            <path d="M20 11a9 9 0 0 0-18 0" />
          </g>

          {/* --- MAIN 320x320 PATTERN WITH 45-DEGREE DIAGONAL ROTATION --- */}
          <pattern
            id="cangu-diagonal-pattern"
            width="320"
            height="320"
            patternTransform="rotate(-45)"
            patternUnits="userSpaceOnUse"
          >
            {/* 1. WHISPER-QUIET DIAGONAL GRID (Hairline 0.5px, opacity 0.22) */}
            <g opacity="0.22" stroke="currentColor" strokeWidth="0.5" strokeDasharray="3 3">
              {/* Grid lines running at 80px intervals (forming a diagonal wireframe) */}
              <line x1="0" y1="0" x2="320" y2="0" />
              <line x1="0" y1="80" x2="320" y2="80" />
              <line x1="0" y1="160" x2="320" y2="160" />
              <line x1="0" y1="240" x2="320" y2="240" />
              <line x1="0" y1="320" x2="320" y2="320" />

              <line x1="0" y1="0" x2="0" y2="320" />
              <line x1="80" y1="0" x2="80" y2="320" />
              <line x1="160" y1="0" x2="160" y2="320" />
              <line x1="240" y1="0" x2="240" y2="320" />
              <line x1="320" y1="0" x2="320" y2="320" />
            </g>

            {/* 2. Subtle coordinate nodes at grid junctions */}
            <g opacity="0.3" fill="currentColor">
              <circle cx="0" cy="0" r="1.2" />
              <circle cx="80" cy="80" r="1.2" />
              <circle cx="160" cy="0" r="1.2" />
              <circle cx="160" cy="160" r="1.2" />
              <circle cx="240" cy="80" r="1.2" />
              <circle cx="240" cy="240" r="1.2" />
              <circle cx="320" cy="0" r="1.2" />
              <circle cx="320" cy="160" r="1.2" />
              <circle cx="320" cy="320" r="1.2" />
              <circle cx="0" cy="160" r="1.2" />
              <circle cx="0" cy="320" r="1.2" />
            </g>

            {/* 3. PROMINENT DOMAIN ICONS (Hero of the pattern, perfectly upright) */}
            <g transform="translate(29, 29) rotate(45 11 11)">
              <use href="#pat-kangaroo" />
            </g>
            <g transform="translate(109, 29) rotate(45 11 11)">
              <use href="#pat-shield-lock" />
            </g>
            <g transform="translate(189, 29) rotate(45 11 11)">
              <use href="#pat-coins" />
            </g>
            <g transform="translate(269, 29) rotate(45 11 11)">
              <use href="#pat-key" />
            </g>
            <g transform="translate(29, 109) rotate(45 11 11)">
              <use href="#pat-scale" />
            </g>
            <g transform="translate(109, 109) rotate(45 11 11)">
              <use href="#pat-swap" />
            </g>
            <g transform="translate(189, 109) rotate(45 11 11)">
              <use href="#pat-clock" />
            </g>
            <g transform="translate(269, 109) rotate(45 11 11)">
              <use href="#pat-user-buyer" />
            </g>
            <g transform="translate(29, 189) rotate(45 11 11)">
              <use href="#pat-hash" />
            </g>
            <g transform="translate(109, 189) rotate(45 11 11)">
              <use href="#pat-layers" />
            </g>
            <g transform="translate(189, 189) rotate(45 11 11)">
              <use href="#pat-file-check" />
            </g>
            <g transform="translate(269, 189) rotate(45 11 11)">
              <use href="#pat-building" />
            </g>
            <g transform="translate(29, 269) rotate(45 11 11)">
              <use href="#pat-terminal" />
            </g>
            <g transform="translate(109, 269) rotate(45 11 11)">
              <use href="#pat-database" />
            </g>
            <g transform="translate(189, 269) rotate(45 11 11)">
              <use href="#pat-zap" />
            </g>
            <g transform="translate(269, 269) rotate(45 11 11)">
              <use href="#pat-fingerprint" />
            </g>
          </pattern>
        </defs>

        <rect width="100%" height="100%" fill="url(#cangu-diagonal-pattern)" />
      </svg>
    </div>
  );
}
