/**
 * Keller Systems — Direct Electronic Chat & Rapid Inquiry Channel
 * Compliance: § 5 Abs. 1 Nr. 2 DDG (formerly TMG) & EuGH C-298/07
 * Zero-Data Architecture: 100% Client-Side, No Cookies, No Remote Trackers
 */
(function() {
  'use strict';

  function initKellerChat() {
    if (document.getElementById('kellerChatDialog')) return;

    // Create backdrop
    var backdrop = document.createElement('div');
    backdrop.id = 'kellerChatBackdrop';
    backdrop.className = 'keller-chat-backdrop';
    document.body.appendChild(backdrop);

    // Create modal dialog
    var dialog = document.createElement('div');
    dialog.id = 'kellerChatDialog';
    dialog.className = 'keller-chat-dialog';
    dialog.setAttribute('role', 'dialog');
    dialog.setAttribute('aria-modal', 'true');
    dialog.setAttribute('aria-labelledby', 'kellerChatTitle');

    dialog.innerHTML = [
      '<div class="keller-chat-head">',
      '  <div class="keller-chat-title-group">',
      '    <span class="chat-status-dot" aria-hidden="true"></span>',
      '    <div>',
      '      <h3 class="keller-chat-title" id="kellerChatTitle">Direkt-Chat</h3>',
      '      <span class="keller-chat-subtitle">&sect; 5 Abs. 1 Nr. 2 DDG Direktkanal</span>',
      '    </div>',
      '  </div>',
      '  <button type="button" class="keller-chat-close" id="kellerChatClose" aria-label="Chat schließen">&times;</button>',
      '</div>',
      '<div class="keller-chat-body" id="kellerChatMessages">',
      '  <div class="keller-chat-badge">',
      '    <span>&bull; Zero-Data: Keine Tracker &bull; Antwort i. d. R. &lt; 60 Min. werktags &bull;</span>',
      '  </div>',
      '  <div class="keller-chat-bubble is-agent">',
      '    <strong>Keller Systems:</strong><br>',
      '    Willkommen! Sie nutzen unseren direkten elektronischen Kontaktkanal gem. &sect;&nbsp;5 Abs.&nbsp;1 Nr.&nbsp;2 DDG. Wie können wir Ihnen helfen?',
      '  </div>',
      '</div>',
      '<form class="keller-chat-form" id="kellerChatForm">',
      '  <div class="keller-chat-field">',
      '    <label class="keller-chat-label" for="kellerChatEmail">Ihre E-Mail-Adresse für die Rückmeldung:</label>',
      '    <input class="keller-chat-input" id="kellerChatEmail" type="email" required placeholder="name@example.com" autocomplete="email">',
      '  </div>',
      '  <div class="keller-chat-field">',
      '    <label class="keller-chat-label" for="kellerChatMessage">Ihre Nachricht:</label>',
      '    <textarea class="keller-chat-textarea" id="kellerChatMessage" required placeholder="Nachricht oder Frage eingeben..."></textarea>',
      '  </div>',
      '  <div class="keller-chat-actions">',
      '    <span class="keller-chat-privacy-hint">Direktübertragung an info@kellersystems.dev</span>',
      '    <button type="submit" class="keller-chat-send">Senden &rarr;</button>',
      '  </div>',
      '</form>'
    ].join('\n');

    document.body.appendChild(dialog);

    // Create floating trigger button if not present
    var floatBtn = document.createElement('button');
    floatBtn.id = 'kellerFloatingChat';
    floatBtn.className = 'keller-chat-floating-btn';
    floatBtn.setAttribute('type', 'button');
    floatBtn.setAttribute('aria-label', 'Direkt-Chat öffnen');
    floatBtn.innerHTML = '<span class="chat-status-dot" aria-hidden="true"></span><span>Direkt-Chat</span>';
    document.body.appendChild(floatBtn);

    function openChat() {
      dialog.classList.add('is-open');
      backdrop.classList.add('is-open');
      floatBtn.style.display = 'none';
      var emailInput = document.getElementById('kellerChatEmail');
      var msgInput = document.getElementById('kellerChatMessage');
      if (emailInput && !emailInput.value) {
        emailInput.focus();
      } else if (msgInput) {
        msgInput.focus();
      }
    }

    function closeChat() {
      dialog.classList.remove('is-open');
      backdrop.classList.remove('is-open');
      floatBtn.style.display = 'flex';
    }

    // Event listeners
    backdrop.addEventListener('click', closeChat);
    document.getElementById('kellerChatClose').addEventListener('click', closeChat);

    floatBtn.addEventListener('click', openChat);

    // Any element with id="openChatBtn" or class="legal-chat-btn"
    document.addEventListener('click', function(e) {
      var target = e.target.closest('#openChatBtn, .legal-chat-btn, [data-open-chat]');
      if (target) {
        e.preventDefault();
        openChat();
      }
    });

    document.addEventListener('keydown', function(e) {
      if (e.key === 'Escape' && dialog.classList.contains('is-open')) {
        closeChat();
      }
    });

    // Form submit
    var form = document.getElementById('kellerChatForm');
    form.addEventListener('submit', function(e) {
      e.preventDefault();
      var email = document.getElementById('kellerChatEmail').value.trim();
      var message = document.getElementById('kellerChatMessage').value.trim();
      if (!email || !message) return;

      var messagesContainer = document.getElementById('kellerChatMessages');

      // User bubble
      var userBubble = document.createElement('div');
      userBubble.className = 'keller-chat-bubble is-user';
      userBubble.textContent = message;
      messagesContainer.appendChild(userBubble);

      // System confirmation bubble
      var now = new Date();
      var timeStr = now.toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
      var confirmBubble = document.createElement('div');
      confirmBubble.className = 'keller-chat-bubble is-system';
      confirmBubble.innerHTML = '<strong>&check; Anfrage erfasst (' + timeStr + ' Uhr):</strong><br>' +
        'Ihre Nachricht wird direkt an unser R&amp;D-Team übermittelt. Wir antworten werktags i.&nbsp;d.&nbsp;R. innerhalb von 60 Minuten an <em>' +
        email.replace(/</g, '&lt;') + '</em>.<br><br>' +
        '<small>Kopie zur Sofort-Übertragung: <a href="mailto:info@kellersystems.dev?subject=' +
        encodeURIComponent('Direktanfrage über Portal-Chat (' + email + ')') +
        '&body=' + encodeURIComponent('Absender: ' + email + '\n\nNachricht:\n' + message) +
        '" style="text-decoration:underline; font-weight:600;">Hier klicken zum Öffnen im E-Mail-Programm</a></small>';

      messagesContainer.appendChild(confirmBubble);
      messagesContainer.scrollTop = messagesContainer.scrollHeight;

      // Reset message field
      document.getElementById('kellerChatMessage').value = '';

      // Trigger mailto fallback for direct client delivery
      var mailtoUrl = 'mailto:info@kellersystems.dev?subject=' +
        encodeURIComponent('Portal-Chat Direct Inquiry from ' + email) +
        '&body=' + encodeURIComponent('Absender: ' + email + '\n\nNachricht:\n' + message);
      
      var mailtoLink = document.createElement('a');
      mailtoLink.href = mailtoUrl;
      mailtoLink.style.display = 'none';
      document.body.appendChild(mailtoLink);
      mailtoLink.click();
      document.body.removeChild(mailtoLink);
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initKellerChat);
  } else {
    initKellerChat();
  }
})();
