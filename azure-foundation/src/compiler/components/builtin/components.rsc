/* Styles par defaut des composants d'Azure (theme sombre). Places AVANT
   la feuille de l'app : une regle de l'app sur le meme selecteur gagne.
   Chaque composant a sa classe `az-...` ; les variantes sont des classes
   en plus : <badge.succes>, <alert.danger>, <btn.primary>. */

/* ------------------------------------------------------------ champs */
checkbox { color: #e7e5e1; background-color: #1c1b19; font-size: 14px; }
radio { color: #e7e5e1; background-color: #1c1b19; font-size: 14px; }
switch { color: #e7e5e1; background-color: #2e2b27; font-size: 14px; }
slider { background-color: #2e2b27; }
progress { background-color: #262420; }
select { color: #e7e5e1; background-color: #171615; font-size: 14px; }
segmented { color: #b8b5ae; background-color: #1c1b19; font-size: 13px; }
rating { accent-color: #fbbf24; background-color: #2e2b27; color: #b8b5ae; font-size: 13px; }
input { width: 260px; height: 36px; background-color: #141312; color: #e9e7e3; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 8px; }
input:hover { background-color: #171615; }
input:focus { background-color: #1a1917; }
.az-error { color: #fca5a5; background-color: rgba(248, 113, 113, 0.12); padding: 4px 8px; border-radius: 6px; font-size: 13px; }

/* ------------------------------------------------------ mise en page */
.az-row { display: flex; flex-direction: row; align-items: center; gap: 12px; flex-wrap: wrap; }
.az-column { display: flex; flex-direction: column; gap: 8px; }
.az-stack { display: flex; flex-direction: column; gap: 16px; }
.az-center { display: flex; flex-direction: column; align-items: center; justify-content: center; }
.az-grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 14px; }
.az-grid.az-cols-1 { grid-template-columns: 1fr; }
.az-grid.az-cols-2 { grid-template-columns: 1fr 1fr; }
.az-grid.az-cols-4 { grid-template-columns: 1fr 1fr 1fr 1fr; }
.az-spacer { flex-grow: 1; height: 16px; }
.az-divider { height: 1px; flex-shrink: 0; margin: 8px 0; background-color: rgba(255, 255, 255, 0.08); }
.az-section { display: flex; flex-direction: column; gap: 10px; margin-bottom: 18px; }
.az-section-title { color: #f3f1ed; font-size: 17px; font-weight: 600; }
.az-section-desc { color: #8a877f; font-size: 13px; }
.az-page { display: flex; flex-direction: column; gap: 14px; padding: 28px 36px; }
.az-page-title { color: #f3f1ed; font-size: 28px; font-weight: 700; }
.az-page-desc { color: #8a877f; font-size: 15px; }
.az-navbar { display: flex; flex-direction: row; align-items: center; gap: 10px; height: 56px; padding: 0 20px; flex-shrink: 0; background-color: #0f0e0d; }
.az-navbar-title { color: #f3f1ed; font-size: 17px; font-weight: 700; margin-right: 18px; }
.az-footer { display: flex; flex-direction: row; gap: 12px; padding: 14px 20px; color: #6f6c66; font-size: 12px; background-color: #0f0e0d; }

/* ------------------------------------------------------------ cartes */
.az-card { display: flex; flex-direction: column; gap: 8px; padding: 16px 18px; background-color: #141312; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 12px; }
.az-card-title { color: #f3f1ed; font-size: 16px; font-weight: 600; }
.az-card-desc { color: #8a877f; font-size: 13px; }
.az-card-body { display: flex; flex-direction: column; gap: 8px; }
.az-card-foot { color: #6f6c66; font-size: 12px; margin-top: 4px; }
.az-stat { display: flex; flex-direction: column; gap: 4px; padding: 16px 18px; background-color: #141312; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 12px; }
.az-stat-label { color: #8a877f; font-size: 12px; font-weight: 600; }
.az-stat-value { color: #f3f1ed; font-size: 26px; font-weight: 700; }
.az-stat-detail { color: #8a877f; font-size: 12px; }
.az-stat-detail.hausse { color: #6ee7a0; }
.az-stat-detail.baisse { color: #fca5a5; }
.az-feature { display: flex; flex-direction: column; gap: 8px; padding: 18px; background-color: #141312; border-radius: 12px; }
.az-feature-icon { width: 36px; height: 36px; border-radius: 10px; background: linear-gradient(135deg, #c9a878 0%, #9c7b55 100%); }
.az-feature-title { color: #f3f1ed; font-size: 15px; font-weight: 600; }
.az-feature-desc { color: #8a877f; font-size: 13px; }
.az-price { display: flex; flex-direction: column; gap: 8px; padding: 20px; background-color: #141312; border: 1px solid rgba(201, 168, 120, 0.25); border-radius: 14px; }
.az-price-name { color: #e0cfb3; font-size: 13px; font-weight: 700; }
.az-price-row { display: flex; flex-direction: row; align-items: flex-end; gap: 6px; }
.az-price-amount { color: #f3f1ed; font-size: 30px; font-weight: 700; }
.az-price-period { color: #8a877f; font-size: 13px; margin-bottom: 6px; }
.az-price-item { color: #d6d3cc; font-size: 13px; }
.az-hero { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 40px 24px; background: linear-gradient(135deg, rgba(201, 168, 120, 0.16) 0%, rgba(201, 168, 120, 0.10) 100%); border-radius: 16px; }
.az-hero-title { color: #f3f1ed; font-size: 32px; font-weight: 700; text-align: center; }
.az-hero-desc { color: #b8b5ae; font-size: 15px; text-align: center; }
.az-hero-actions { display: flex; flex-direction: row; gap: 10px; margin-top: 6px; }

/* ----------------------------------------------- petits elements */
.az-badge { padding: 2px 10px; border-radius: 999px; font-size: 12px; font-weight: 600; background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; }
.az-badge.succes { background-color: rgba(74, 222, 128, 0.12); color: #6ee7a0; }
.az-badge.danger { background-color: rgba(248, 113, 113, 0.12); color: #fca5a5; }
.az-badge.attention { background-color: rgba(251, 191, 36, 0.12); color: #fcd34d; }
.az-badge.neutre { background-color: rgba(255, 255, 255, 0.06); color: #b8b5ae; }
.az-tag { padding: 3px 10px; border-radius: 6px; font-size: 12px; background-color: #1f1e1b; color: #d6d3cc; border: 1px solid rgba(255, 255, 255, 0.08); }
.az-chip { display: flex; flex-direction: row; align-items: center; gap: 6px; padding: 4px 6px 4px 12px; border-radius: 999px; background-color: rgba(201, 168, 120, 0.10); }
.az-chip-text { color: #ecdcc0; font-size: 13px; }
.az-chip-x { padding: 2px 8px; border-radius: 999px; background-color: rgba(255, 255, 255, 0.08); color: #b8b5ae; font-size: 11px; }
.az-avatar { display: flex; flex-direction: column; align-items: center; justify-content: center; width: 40px; height: 40px; border-radius: 999px; background: linear-gradient(135deg, #c9a878 0%, #9c7b55 100%); flex-shrink: 0; }
.az-avatar-text { color: #ffffff; font-size: 15px; font-weight: 700; }
.az-kbd { padding: 1px 7px; border-radius: 5px; background-color: #1f1e1b; border: 1px solid rgba(255, 255, 255, 0.14); color: #e7e5e1; font-size: 12px; font-weight: 600; }
.az-code { padding: 12px 14px; border-radius: 10px; background-color: #0c0c0b; border: 1px solid rgba(255, 255, 255, 0.06); }
.az-code-text { color: #c3e88d; font-size: 13px; white-space: pre; }
.az-quote { display: flex; flex-direction: column; gap: 6px; padding: 10px 16px; border-left: 3px solid #c9a878; background-color: rgba(201, 168, 120, 0.05); }
.az-quote-text { color: #e7e5e1; font-size: 15px; }
.az-quote-author { color: #8a877f; font-size: 12px; }
.az-skeleton { display: flex; flex-direction: column; gap: 8px; }
.az-skel-line { height: 12px; width: 70%; border-radius: 6px; background-color: #1f1e1b; }
.az-skel-line.wide { width: 100%; }
.az-skel-line.short { width: 40%; }

/* --------------------------------------------------------- alertes */
.az-alert { display: flex; flex-direction: column; gap: 4px; padding: 12px 16px; border-radius: 10px; background-color: rgba(201, 168, 120, 0.10); border: 1px solid rgba(201, 168, 120, 0.30); }
.az-alert-title { color: #ecdcc0; font-size: 14px; font-weight: 700; }
.az-alert-text { color: #d6d3cc; font-size: 13px; }
.az-alert.succes { background-color: rgba(74, 222, 128, 0.08); border-color: rgba(74, 222, 128, 0.30); }
.az-alert.succes .az-alert-title { color: #6ee7a0; }
.az-alert.attention { background-color: rgba(251, 191, 36, 0.08); border-color: rgba(251, 191, 36, 0.30); }
.az-alert.attention .az-alert-title { color: #fcd34d; }
.az-alert.danger { background-color: rgba(248, 113, 113, 0.08); border-color: rgba(248, 113, 113, 0.30); }
.az-alert.danger .az-alert-title { color: #fca5a5; }
.az-toast { display: flex; flex-direction: row; gap: 12px; width: 340px; padding: 12px; border-radius: 12px; background-color: #1c1b19; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45); }
.az-toast-bar { width: 4px; border-radius: 4px; background-color: #c9a878; flex-shrink: 0; }
.az-toast.succes .az-toast-bar { background-color: #4ade80; }
.az-toast.danger .az-toast-bar { background-color: #f87171; }
.az-toast-body { display: flex; flex-direction: column; gap: 3px; }
.az-toast-title { color: #f3f1ed; font-size: 14px; font-weight: 600; }
.az-toast-text { color: #b8b5ae; font-size: 13px; }
.az-empty { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 32px; border: 1px dashed rgba(255, 255, 255, 0.12); border-radius: 12px; }
.az-empty-icon { width: 44px; height: 44px; border-radius: 999px; background-color: #1f1e1b; }
.az-empty-title { color: #f3f1ed; font-size: 15px; font-weight: 600; }
.az-empty-text { color: #8a877f; font-size: 13px; text-align: center; }

/* --------------------------------------------------------- boutons */
.az-btn { padding: 8px 16px; border-radius: 8px; background-color: #1f1e1b; border: 1px solid rgba(255, 255, 255, 0.08); color: #e7e5e1; font-size: 14px; font-weight: 500; }
.az-btn:hover { background-color: #282622; }
.az-btn.primary { background-color: #c9a878; border-color: #c9a878; color: #1a1712; }
.az-btn.primary:hover { background-color: #d8b98a; }
.az-btn.danger { background-color: rgba(248, 113, 113, 0.14); border-color: rgba(248, 113, 113, 0.35); color: #fca5a5; }
.az-btn.ghost { background-color: transparent; border-color: transparent; color: #e0cfb3; }
.az-btn.small { padding: 4px 10px; font-size: 12px; }
.az-link { padding: 0; background-color: transparent; color: #e0cfb3; font-size: 14px; text-align: left; }
.az-link:hover { color: #ecdcc0; }

/* ---------------------------------------------------- navigation */
.az-list { display: flex; flex-direction: column; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 12px; background-color: #141312; }
.az-list-item { display: flex; flex-direction: row; align-items: center; gap: 12px; padding: 12px 16px; border-bottom: 1px solid rgba(255, 255, 255, 0.05); }
.az-list-main { display: flex; flex-direction: column; gap: 2px; flex-grow: 1; }
.az-list-title { color: #f3f1ed; font-size: 14px; font-weight: 500; }
.az-list-desc { color: #8a877f; font-size: 12px; }
.az-menu { display: flex; flex-direction: column; gap: 2px; }
.az-menu-item { padding: 8px 12px; border-radius: 8px; background-color: transparent; color: #8a877f; font-size: 14px; text-align: left; }
.az-menu-item:hover { background-color: rgba(255, 255, 255, 0.05); color: #f3f1ed; }
.az-menu-item.on { background-color: rgba(201, 168, 120, 0.12); color: #e0cfb3; font-weight: 600; }
.az-tabs { display: flex; flex-direction: row; gap: 4px; padding: 4px; border-radius: 10px; background-color: #141312; }
.az-tab { padding: 7px 14px; border-radius: 7px; background-color: transparent; color: #8a877f; font-size: 13px; font-weight: 500; }
.az-tab:hover { color: #f3f1ed; }
.az-tab.on { background-color: #282622; color: #f3f1ed; font-weight: 600; }
.az-breadcrumb { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.az-crumb { color: #e0cfb3; font-size: 13px; }
.az-crumb-sep { color: #4d4a45; font-size: 13px; }
.az-steps { display: flex; flex-direction: row; gap: 18px; align-items: center; }
.az-step { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.az-step-dot { width: 26px; height: 26px; border-radius: 999px; background-color: #1f1e1b; color: #8a877f; font-size: 12px; font-weight: 700; text-align: center; line-height: 26px; }
.az-step-dot.done { background-color: rgba(74, 222, 128, 0.18); color: #6ee7a0; }
.az-step-dot.current { background-color: #c9a878; color: #1a1712; }
.az-step-label { color: #d6d3cc; font-size: 13px; }
.az-pagination { display: flex; flex-direction: row; gap: 4px; }
.az-pagelink { width: 34px; height: 34px; padding: 0; border-radius: 8px; background-color: #171615; color: #b8b5ae; font-size: 13px; }
.az-pagelink:hover { background-color: #22201d; }
.az-pagelink.on { background-color: #c9a878; color: #1a1712; font-weight: 700; }
.az-timeline { display: flex; flex-direction: column; gap: 14px; }
.az-timeline-item { display: flex; flex-direction: row; gap: 12px; }
.az-timeline-dot { width: 10px; height: 10px; margin-top: 5px; border-radius: 999px; background-color: #c9a878; flex-shrink: 0; }
.az-timeline-body { display: flex; flex-direction: column; gap: 3px; flex-grow: 1; }
.az-timeline-head { display: flex; flex-direction: row; gap: 10px; align-items: center; }
.az-timeline-title { color: #f3f1ed; font-size: 14px; font-weight: 600; }
.az-timeline-date { color: #6f6c66; font-size: 12px; }
.az-accordion { display: flex; flex-direction: column; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 10px; background-color: #141312; }
.az-accordion-head { padding: 12px 16px; background-color: transparent; color: #f3f1ed; font-size: 14px; font-weight: 500; text-align: left; }
.az-accordion-head.on { color: #e0cfb3; }
.az-accordion-body { display: flex; flex-direction: column; gap: 6px; padding: 0 16px 14px 16px; color: #b8b5ae; font-size: 13px; }

/* ------------------------------------------------------- tableaux */
.az-table { display: flex; flex-direction: column; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 10px; background-color: #141312; }
.az-tr { display: flex; flex-direction: row; border-bottom: 1px solid rgba(255, 255, 255, 0.05); }
.az-th { flex-grow: 1; flex-basis: 0; padding: 10px 14px; background-color: #181716; }
.az-th-text { color: #8a877f; font-size: 12px; font-weight: 700; }
.az-td { flex-grow: 1; flex-basis: 0; padding: 10px 14px; color: #e7e5e1; font-size: 13px; }
.az-info { display: flex; flex-direction: row; gap: 12px; padding: 6px 0; }
.az-info-term { width: 160px; color: #8a877f; font-size: 13px; flex-shrink: 0; }
.az-info-value { color: #f3f1ed; font-size: 13px; }

/* ---------------------------------------------------- formulaires */
.az-form { display: flex; flex-direction: column; gap: 14px; }
.az-field { display: flex; flex-direction: column; gap: 6px; }
.az-field-label { color: #d6d3cc; font-size: 13px; font-weight: 600; }
.az-field-help { color: #6f6c66; font-size: 12px; }
.az-field-error { color: #fca5a5; font-size: 12px; }
.az-meter { display: flex; flex-direction: column; gap: 6px; }
.az-meter-head { display: flex; flex-direction: row; justify-content: space-between; }
.az-meter-label { color: #d6d3cc; font-size: 13px; }
.az-meter-value { color: #8a877f; font-size: 12px; }

/* -------------------------------------------- couches (position: fixed) */
.az-modal-backdrop { position: fixed; top: 0; right: 0; bottom: 0; left: 0; z-index: 100; display: flex; flex-direction: column; align-items: center; justify-content: center; background-color: rgba(8, 8, 7, 0.62); }
.az-modal { display: flex; flex-direction: column; gap: 12px; width: 460px; max-width: 90%; padding: 18px 20px; background-color: #161514; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 14px; box-shadow: 0 18px 50px rgba(0, 0, 0, 0.55); }
.az-modal-head { display: flex; flex-direction: row; align-items: center; gap: 10px; }
.az-modal-title { flex-grow: 1; color: #f3f1ed; font-size: 17px; font-weight: 700; }
.az-modal-close { width: 30px; height: 30px; padding: 0; border-radius: 8px; background-color: rgba(255, 255, 255, 0.06); color: #b8b5ae; font-size: 13px; }
.az-modal-close:hover { background-color: rgba(255, 255, 255, 0.12); }
.az-modal-body { display: flex; flex-direction: column; gap: 10px; color: #d6d3cc; font-size: 14px; }
.az-drawer-backdrop { position: fixed; top: 0; right: 0; bottom: 0; left: 0; z-index: 90; display: flex; flex-direction: row; justify-content: flex-end; background-color: rgba(8, 8, 7, 0.5); }
.az-drawer { display: flex; flex-direction: column; gap: 14px; width: 360px; padding: 18px 20px; background-color: #161514; border-left: 1px solid rgba(255, 255, 255, 0.08); }
.az-drawer-body { display: flex; flex-direction: column; gap: 10px; flex-grow: 1; min-height: 0; overflow-y: auto; }
.az-toasts { position: fixed; right: 20px; bottom: 20px; z-index: 200; display: flex; flex-direction: column; gap: 10px; }
.az-banner { display: flex; flex-direction: row; align-items: center; gap: 12px; padding: 10px 16px; background: linear-gradient(90deg, #6e5a44 0%, #4a3d30 100%); }
.az-banner-text { flex-grow: 1; color: #ffffff; font-size: 13px; font-weight: 600; }
.az-banner-close { padding: 2px 9px; border-radius: 6px; background-color: rgba(255, 255, 255, 0.18); color: #ffffff; font-size: 12px; }

/* Barre d'outils du texte riche : <richbar pour="id-du-richtext"> */
.az-richbar { display: flex; align-items: center; gap: 2px; padding: 4px; border-radius: 8px; background-color: rgba(255, 255, 255, 0.04); }
.az-rt, .az-rt-c { height: 28px; min-width: 28px; padding: 0 8px; border-radius: 6px; background-color: transparent; color: #d6d3cc; font-size: 13px; }
.az-rt:hover, .az-rt-c:hover { background-color: rgba(255, 255, 255, 0.08); }
.az-rt-gras { font-weight: 700; }
.az-rt-italique { font-style: italic; }
.az-rt-souligne { text-decoration: underline; }
.az-rt-barre { text-decoration: line-through; }
.az-rt-code { font-family: monospace; }
.az-rt-sep { width: 1px; height: 18px; margin: 0 6px; background-color: rgba(255, 255, 255, 0.12); }
.az-rt-c { font-weight: 700; }
.az-rt-rouge { color: #e06c75; }
.az-rt-orange { color: #d19a66; }
.az-rt-jaune { color: #e5c07b; }
.az-rt-vert { color: #98c379; }
.az-rt-violet { color: #c678dd; }
.az-rt-gris { color: #8a8a8a; }
richtext { min-height: 120px; padding: 6px; border-radius: 8px; background-color: rgba(255, 255, 255, 0.03); color: #e8e4dc; }
