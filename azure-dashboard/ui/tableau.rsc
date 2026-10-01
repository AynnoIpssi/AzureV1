/* Azure Dashboard : le theme d'Azure Docs - gris chauds, accent sable,
   sauge et terracotta pour les etats, pas de bleu. */
.app { display: flex; flex-direction: column; height: 100%; background-color: #121212; color: #c9c6bf; font-size: 14px; }
.hr { height: 1px; flex-shrink: 0; background-color: rgba(255, 255, 255, 0.07); }

.topbar { display: flex; flex-direction: row; align-items: center; gap: 6px; height: 60px; padding: 0 24px; flex-shrink: 0; background-color: #161616; border-bottom: 1px solid #272727; }
.logo { width: 28px; height: 28px; margin-right: 10px; background: linear-gradient(135deg, #c9a878 0%, #9c7b55 100%); border-radius: 8px; }
.brand { color: #f0eeea; font-size: 18px; font-weight: 700; margin-right: 28px; }
.nav { padding: 8px 14px; background-color: transparent; border-radius: 8px; color: #8a877f; font-size: 14px; font-weight: 500; }
.nav:hover { background-color: rgba(255, 255, 255, 0.05); color: #f0eeea; }
.nav.on { background-color: rgba(201, 168, 120, 0.12); color: #e0cfb3; font-weight: 600; }
.pill { margin-left: auto; padding: 4px 12px; background-color: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 999px; color: #8a877f; font-size: 12px; }

.page { display: flex; flex-direction: column; gap: 10px; flex-grow: 1; min-height: 0; padding: 28px 36px; overflow-y: auto; }
.h { color: #f0eeea; font-size: 26px; font-weight: 700; }
.lead { color: #8a877f; font-size: 14px; margin-bottom: 8px; }
.section { color: #f0eeea; font-size: 16px; font-weight: 600; margin-top: 14px; }
.muted { color: #6f6c66; font-size: 13px; }
.line { color: #c9c6bf; font-size: 13px; }
.label { color: #8a877f; font-size: 12px; font-weight: 600; margin-top: 6px; }

.cards { display: flex; flex-direction: row; flex-wrap: wrap; gap: 14px; }
.card { display: flex; flex-direction: column; gap: 6px; width: 340px; padding: 16px 18px; background-color: #181818; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 12px; }
.page .card { flex-shrink: 0; }
.card.wide { width: 640px; }
.row { display: flex; flex-direction: row; align-items: center; gap: 10px; }
.name { color: #f0eeea; font-size: 16px; font-weight: 600; }

.badge { padding: 2px 10px; border-radius: 999px; font-size: 12px; font-weight: 600; }
.badge.on { background-color: rgba(155, 176, 143, 0.14); color: #b9cbb0; }
.badge.off { background-color: rgba(201, 138, 107, 0.12); color: #e0ad94; }
.badge.public { background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; }
.badge.private { background-color: rgba(255, 255, 255, 0.06); color: #b8b5ae; }
.badge.warn { background-color: rgba(214, 170, 90, 0.14); color: #e3bf7a; }

.btn { padding: 7px 14px; margin-top: 6px; background-color: #1e1e1e; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; color: #e7e5e1; font-size: 13px; font-weight: 500; }
.btn:hover { background-color: #26241f; }
.btn:active { background-color: #c9a878; color: #1a1712; }
.btn.danger { background-color: rgba(201, 138, 107, 0.14); border: 1px solid rgba(201, 138, 107, 0.35); color: #e0ad94; }
.btn.danger:hover { background-color: rgba(201, 138, 107, 0.24); }
.btn.danger:active { background-color: #c98a6b; color: #1a1712; }
.alerte { display: flex; flex-direction: row; align-items: center; gap: 12px; padding: 10px 14px; flex-shrink: 0; background-color: rgba(201, 138, 107, 0.12); border: 1px solid rgba(201, 138, 107, 0.35); border-radius: 10px; }
.alerte-texte { flex-grow: 1; color: #e0ad94; font-size: 13px; }
.alerte-x { padding: 4px 12px; background-color: transparent; border: 1px solid rgba(201, 138, 107, 0.35); border-radius: 999px; color: #e0ad94; font-size: 12px; }
.btn.ghost { background-color: transparent; color: #8a877f; }
.btn.small { margin-top: 0; margin-left: auto; padding: 5px 12px; }
.back { padding: 4px 0; background-color: transparent; color: #e0cfb3; font-size: 13px; text-align: left; }

.chips { display: flex; flex-direction: row; flex-wrap: wrap; gap: 8px; }
.chip { display: flex; flex-direction: row; align-items: center; gap: 6px; padding: 4px 6px 4px 12px; background-color: rgba(201, 168, 120, 0.10); border-radius: 999px; }
.chiptext { color: #e0cfb3; font-size: 13px; }
.x { padding: 3px 10px; background-color: rgba(201, 138, 107, 0.14); border-radius: 999px; color: #e0ad94; font-size: 12px; }
.x:hover { background-color: rgba(201, 138, 107, 0.24); }
.add { padding: 4px 12px; background-color: transparent; border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 999px; color: #b8b5ae; font-size: 13px; }
.add:hover { background-color: rgba(255, 255, 255, 0.05); }

.link { display: flex; flex-direction: row; align-items: center; gap: 12px; padding: 12px 16px; background-color: #181818; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 10px; flex-shrink: 0; }
.from { color: #f0eeea; font-size: 14px; font-weight: 600; }
.arrow { color: #c9a878; font-size: 13px; }
.to { color: #f0eeea; font-size: 14px; font-weight: 600; }

.stat { color: #e0cfb3; font-size: 12px; font-weight: 600; }
.error { color: #e0ad94; font-size: 13px; }
.time { color: #6f6c66; font-size: 12px; width: 64px; flex-shrink: 0; }
.eventline { display: flex; flex-direction: row; gap: 10px; align-items: center; }
.badge.erreur { background-color: rgba(201, 138, 107, 0.14); color: #e0ad94; }
.badge.attention { background-color: rgba(214, 170, 90, 0.14); color: #e3bf7a; }
.badge.info { background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; }
.logbox { display: flex; flex-direction: column; gap: 2px; padding: 14px 16px; background-color: #0e0e0e; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 10px; }
.logline { color: #b9cbb0; font-size: 12px; white-space: pre; }

/* Terminal */
.btn.tight { margin-top: 0; padding: 5px 12px; }
.term { display: flex; flex-direction: column; gap: 2px; padding: 14px 16px; min-height: 260px; background-color: #0e0e0e; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 10px; flex-shrink: 0; }
.term.court { min-height: 0; margin-top: 6px; }
.term-cmd { color: #e0cfb3; font-size: 13px; font-weight: 600; white-space: pre; margin-top: 6px; }
.term-out { color: #c9c6bf; font-size: 12px; white-space: pre; }
.term-ok { color: #b9cbb0; font-size: 12px; white-space: pre; }
.term-err { color: #e0ad94; font-size: 12px; white-space: pre; }
.prompt { display: flex; flex-direction: row; align-items: center; gap: 10px; flex-shrink: 0; padding: 8px 10px 8px 14px; background-color: #0e0e0e; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 10px; }
.term-signe { color: #c9a878; font-size: 13px; font-weight: 600; }
.term-saisie { flex-grow: 1; height: 34px; padding: 0 10px; background-color: #161616; border: 1px solid #2c2c2c; border-radius: 8px; color: #e7e5e1; font-size: 13px; }
.term-saisie:focus { border: 1px solid #c9a878; }
.term-attente { color: #e3bf7a; font-size: 13px; }
