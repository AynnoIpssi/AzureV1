/* Feuille commune aux 3 fenetres de la demo shared_window_demo. */
.app {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    background-color: #0b0d14;
    color: #c9cde0;
    font-size: 15px;
}
.note {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 24px;
    background: linear-gradient(135deg, #1b2a4e, #3a1f5c);
    color: #e6e8f5;
    font-size: 15px;
}
.title { font-size: 24px; font-weight: 700; color: #ffffff; margin-bottom: 12px; }
.body { margin-bottom: 20px; }
.badge {
    font-size: 12px;
    color: #9fb4ff;
    margin-bottom: 16px;
}
.send {
    width: 220px;
    height: 40px;
    border-radius: 8px;
    background-color: #3b5bdb;
    color: #ffffff;
    font-weight: 600;
}
.send:hover { background-color: #4c6ef5; }
