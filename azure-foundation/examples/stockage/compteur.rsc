/* Feuille de la demo stockage_demo. */
.app {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 28px;
    background: linear-gradient(160deg, #0d1224, #1a1036);
    color: #c9cde0;
    font-size: 15px;
}
.badge { font-size: 12px; color: #9fb4ff; margin-bottom: 10px; }
.title { font-size: 26px; font-weight: 700; color: #ffffff; margin-bottom: 8px; }
.body { margin-bottom: 22px; color: #a8aecb; }
.chiffre {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 18px;
    margin-bottom: 22px;
    border-radius: 14px;
    background-color: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(159, 180, 255, 0.25);
}
.valeur { font-size: 64px; font-weight: 700; color: #ffffff; text-align: center; }
.legende { font-size: 13px; color: #9fb4ff; text-align: center; }
.actions { display: flex; flex-direction: row; column-gap: 12px; margin-bottom: 18px; }
.plus {
    width: 120px;
    height: 44px;
    border-radius: 10px;
    background-color: #3b5bdb;
    color: #ffffff;
    font-size: 18px;
    font-weight: 700;
}
.plus:hover { background-color: #4c6ef5; }
.zero {
    width: 180px;
    height: 44px;
    border-radius: 10px;
    background-color: rgba(255, 255, 255, 0.08);
    color: #e6e8f5;
    font-weight: 600;
}
.zero:hover { background-color: rgba(255, 255, 255, 0.16); }
.info { font-size: 12px; color: #7d84a8; }
