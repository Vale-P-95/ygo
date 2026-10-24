/** What an info panel is about. It picks the glyph the Archmage writes on his board. */
export type InfoKind = "what" | "how" | "rank" | "stat" | "block" | "tip" | "warn";

export const INFO_GLYPHS: Record<InfoKind, string> = {
  what: "?",
  how: "A+B",
  rank: "1 2 3",
  stat: "%",
  block: "✕",
  tip: "★",
  warn: "!",
};

/**
 * The Archmage who explains an info panel (from the Claude Design handoff).
 * The glyph is chalked on his board; it is always one of our own constants,
 * never user text, so it can go in the markup as it is.
 */
const archmageSvg = (glyph: string) => `<svg viewBox="0 0 230 220" width="230" height="220" style="overflow:visible">
<defs>
<radialGradient id="dfAura" cx="50%" cy="50%" r="50%"><stop offset="0" stop-color="#9b5cff" stop-opacity=".55"/><stop offset="1" stop-color="#9b5cff" stop-opacity="0"/></radialGradient>
<linearGradient id="dfCloak" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3b3fa8"/><stop offset="1" stop-color="#12123a"/></linearGradient>
<linearGradient id="dfHead" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#4a5bd4"/><stop offset="1" stop-color="#1c1f5e"/></linearGradient>
<radialGradient id="dfEye" cx="50%" cy="45%" r="55%"><stop offset="0" stop-color="#fff6c2"/><stop offset=".45" stop-color="#ffc533"/><stop offset="1" stop-color="#d9480f"/></radialGradient>
<radialGradient id="dfOrb" cx="40%" cy="35%" r="60%"><stop offset="0" stop-color="#fff"/><stop offset=".4" stop-color="#7ee8ff"/><stop offset="1" stop-color="#2563eb"/></radialGradient>
</defs>
<ellipse cx="90" cy="120" rx="95" ry="95" fill="url(#dfAura)" style="animation:dfGlow 2.6s ease-in-out infinite"/>
<g style="transform-origin:90px 128px;animation:dfSpin 18s linear infinite" fill="none" stroke="#c9a2ff" stroke-opacity=".55">
<circle cx="90" cy="128" r="78" stroke-width="1.5"/><circle cx="90" cy="128" r="70" stroke-width="1" stroke-dasharray="3 6"/>
<polygon points="90,58 150,163 30,163" stroke-width="1"/><polygon points="90,198 30,93 150,93" stroke-width="1"/>
</g>
<g style="animation:dfFade .4s .35s both">
<g style="animation:dfBob 3s .3s ease-in-out infinite">
<polygon points="146,14 222,14 228,22 228,76 220,84 146,84 140,76 140,22" fill="#120b22" stroke="#e0b341" stroke-width="2.5"/>
<polygon points="150,20 218,20 222,26 222,72 216,78 150,78 146,72 146,26" fill="none" stroke="#e0b341" stroke-opacity=".45"/>
<text x="184" y="57" text-anchor="middle" font-family="Cinzel,Georgia,serif" font-size="${glyph.length > 2 ? 17 : 28}" font-weight="700" fill="#ffe8a3" style="filter:drop-shadow(0 0 5px #ffb020);animation:dfChalk 1.1s .6s both;transform-box:fill-box;transform-origin:center">${glyph}</text>
</g>
</g>
<ellipse cx="88" cy="210" rx="58" ry="6" fill="#000" opacity=".35"/>
<g style="animation:dfBob 2.4s 1s ease-in-out infinite">
<polygon points="26,208 50,124 126,124 150,208 118,200 88,210 58,200" fill="url(#dfCloak)" stroke="#0a0614" stroke-width="2"/>
<path d="M26 208L58 200L88 210L118 200L150 208" fill="none" stroke="#e0b341" stroke-width="3" stroke-linejoin="round"/>
<path d="M44 150l-6 14M132 150l6 14" stroke="#e0b341" stroke-width="1.5" stroke-opacity=".6"/>
<polygon points="50,124 26,170 42,176 56,150" fill="#2c1b52" stroke="#0a0614" stroke-width="1.5"/>
<circle cx="32" cy="172" r="7" fill="#e9c3a0" stroke="#7a4f2e" stroke-width="1.2"/>
<g style="animation:dfGlow 1.4s ease-in-out infinite"><path d="M32 150l2.5 6 6 2.5-6 2.5-2.5 6-2.5-6-6-2.5 6-2.5z" fill="#7ee8ff" style="filter:drop-shadow(0 0 6px #7ee8ff)"/></g>
<g style="transform-origin:138px 160px;animation:dfTap 1.6s 1s ease-in-out infinite">
<path d="M128 206L166 72" stroke="#5a3a1c" stroke-width="6" stroke-linecap="round"/>
<path d="M166 72q-12-8-6-22q6 8 14 4q2 12-8 18z" fill="#5a3a1c"/>
<circle cx="168" cy="62" r="9" fill="url(#dfOrb)" style="filter:drop-shadow(0 0 10px #7ee8ff)"/>
</g>
<polygon points="126,124 146,150 138,162 122,146" fill="#2c1b52" stroke="#0a0614" stroke-width="1.5"/>
<circle cx="138" cy="156" r="7" fill="#e9c3a0" stroke="#7a4f2e" stroke-width="1.2"/>
<ellipse cx="88" cy="94" rx="21" ry="20" fill="#e9c3a0" stroke="#7a4f2e" stroke-width="1.5"/>
<path d="M64 98Q60 132 74 152Q82 170 88 184Q94 170 102 152Q116 132 112 98Q102 112 88 112Q74 112 64 98Z" fill="#eef0f6" stroke="#9aa0b4" stroke-width="1.5"/>
<path d="M76 130q4 14 10 24M100 130q-4 14-10 24M88 118v30" fill="none" stroke="#b9bdcc" stroke-width="1.2"/>
<path d="M88 104q-10-2-20 6q8 2 20-2q12 4 20 2q-10-8-20-6z" fill="#fafbff" stroke="#9aa0b4" stroke-width="1.2"/>
<polygon points="86,94 90,94 88,102" fill="#d9a07c"/>
<g style="transform-box:fill-box;transform-origin:center;animation:dfBlink 4s 1.2s infinite"><circle cx="80" cy="89" r="3" fill="#7ee8ff" style="filter:drop-shadow(0 0 4px #7ee8ff)"/><circle cx="96" cy="89" r="3" fill="#7ee8ff" style="filter:drop-shadow(0 0 4px #7ee8ff)"/></g>
<circle cx="80" cy="89" r="7" fill="none" stroke="#e0b341" stroke-width="2.2"/><circle cx="96" cy="89" r="7" fill="none" stroke="#e0b341" stroke-width="2.2"/><path d="M87 88h2" stroke="#e0b341" stroke-width="2.2"/>
<path d="M70 80q8-6 16 0M90 80q8-6 16 0" fill="none" stroke="#fafbff" stroke-width="4" stroke-linecap="round"/>
<path d="M54 76L80 22Q88 4 108 2Q94 12 94 28L122 76Z" fill="url(#dfHead)" stroke="#0a0614" stroke-width="2.5" stroke-linejoin="round"/>
<path d="M60 68q28 6 56 0" fill="none" stroke="#e0b341" stroke-width="4"/>
<path d="M84 46l2 5 5 2-5 2-2 5-2-5-5-2 5-2z" fill="#ffe8a3"/><circle cx="100" cy="56" r="2" fill="#ffe8a3"/><circle cx="76" cy="60" r="1.6" fill="#ffe8a3"/>
<path d="M98 30a7 7 0 1 0 4 10a5 5 0 1 1-4-10z" fill="#ffe8a3"/>
<ellipse cx="88" cy="76" rx="52" ry="9" fill="#2c1b52" stroke="#0a0614" stroke-width="2.5"/>
</g>
</svg>`;

export function Mascot({ kind, glyph }: { kind: InfoKind; glyph?: string }) {
  return <div className="mascot" aria-hidden="true" dangerouslySetInnerHTML={{ __html: archmageSvg(glyph ?? INFO_GLYPHS[kind]) }} />;
}
