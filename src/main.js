let ws;
let username;
let isAuthenticated = false;

function showStatus(message, isError = false) {
    const statusDiv = document.getElementById('statusMessages');
    statusDiv.textContent = message;
    statusDiv.className = 'status ' + (isError ? 'error' : 'success');
    console.log(message);
}

function enableMessageInput() {
    document.getElementById('message').disabled = false;
    document.getElementById('sendButton').disabled = false;
    isAuthenticated = true;
}

function disableMessageInput() {
    document.getElementById('message').disabled = true;
    document.getElementById('sendButton').disabled = true;
    isAuthenticated = false;
}

function addMessage(message) {
    const messagesDiv = document.getElementById('messages');
    const messageElement = document.createElement('div');
    messageElement.className = 'message';
    messageElement.innerHTML = `<strong>${message.from}:</strong> ${message.content}`;
    messagesDiv.appendChild(messageElement);
    messagesDiv.scrollTop = messagesDiv.scrollHeight;
}

function register() {
    username = document.getElementById('username').value;
    const password = document.getElementById('password').value;

    if (!username || !password) {
        showStatus('Please enter both username and password', true);
        return;
    }

    showStatus('Attempting to register...');
    ws = new WebSocket('ws://localhost:8100');

    ws.onopen = () => {
        showStatus('Connected to server, sending registration...');
        ws.send(JSON.stringify({
            type: "register",
            username: username,
            password: password
        }));
    };

    ws.onmessage = (event) => {
        showStatus('Server response: ' + event.data);
        ws.close();
    };

    ws.onerror = (error) => {
        showStatus('WebSocket error during registration: ' + error, true);
    };

    ws.onclose = () => {
        showStatus('Registration connection closed');
        disableMessageInput();
    };
}

function connect() {
    username = document.getElementById('username').value;
    const password = document.getElementById('password').value;

    if (!username || !password) {
        showStatus('Please enter both username and password', true);
        return;
    }

    showStatus('Attempting to connect...');
    ws = new WebSocket('ws://localhost:8100');

    ws.onopen = () => {
        showStatus('Connected to server, sending login credentials...');
        ws.send(JSON.stringify({
            type: "login",
            username: username,
            password: password
        }));
    };

    ws.onmessage = (event) => {
        console.log('Received message:', event.data);

        if (event.data === 'Authentication failed') {
            showStatus('Authentication failed', true);
            ws.close();
            return;
        }

        if (event.data === 'Authentication successful') {
            showStatus('Successfully connected and authenticated');
            enableMessageInput();
            return;
        }

        try {
            const message = JSON.parse(event.data);
            console.log('Parsed message:', message);
            addMessage(message);
        } catch (e) {
            console.log('Error parsing message:', e);
            showStatus('Received: ' + event.data);
        }
    };

    ws.onerror = (error) => {
        showStatus('WebSocket error: ' + error, true);
    };

    ws.onclose = () => {
        showStatus('Connection closed');
        disableMessageInput();
    };
}

function sendMessage() {
    if (!isAuthenticated) {
        showStatus('Not authenticated. Please connect first.', true);
        return;
    }

    const messageInput = document.getElementById('message');
    const content = messageInput.value.trim();
    if (!content) return;

    const message = {
        from: username,
        content: content,
        timestamp: Date.now()
    };

    try {
        console.log('Sending message:', message);
        ws.send(JSON.stringify(message));
        messageInput.value = '';
        // Remove this line to prevent duplication
        // addMessage(message);
    } catch (error) {
        showStatus('Error sending message: ' + error, true);
    }
}

// Add enter key support for message sending
document.getElementById('message').addEventListener('keypress', function(e) {
    if (e.key === 'Enter') {
        sendMessage();
    }
});