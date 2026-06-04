class WebChatPublisher {
    constructor() {
        this.enabled = false;
        this.apiUrl = '';
        this.apiKey = '';
        this.roomId = null;
        this.shareUrl = '';
        this.seq = 1;
        this.failed = false;
    }

    configure({ enabled, apiUrl, apiKey }) {
        this.enabled = Boolean(enabled);
        this.apiUrl = (apiUrl || '').trim().replace(/\/+$/, '');
        this.apiKey = (apiKey || '').trim();
    }

    async createSession(config) {
        this.configure(config);
        this.roomId = null;
        this.shareUrl = '';
        this.seq = 1;
        this.failed = false;

        if (!this.enabled) return null;
        if (!this.apiUrl) throw new Error('Web Chat API URL is required');
        if (!this.apiKey) throw new Error('Web Chat API key is required');

        const response = await fetch(`${this.apiUrl}/api/sessions`, {
            method: 'POST',
            headers: { 'x-api-key': this.apiKey },
        });

        if (!response.ok) {
            throw new Error(`create room failed: HTTP ${response.status}`);
        }

        const data = await response.json();
        this.roomId = data.room_id;
        this.shareUrl = data.share_url;
        return data;
    }

    async publish(event) {
        if (!this.enabled || !this.roomId || this.failed) return false;

        const payload = {
            ...event,
            seq: event.seq || this.seq++,
            ts: event.ts || Date.now(),
        };

        try {
            const response = await fetch(`${this.apiUrl}/api/sessions/${this.roomId}/events`, {
                method: 'POST',
                headers: {
                    'content-type': 'application/json',
                    'x-api-key': this.apiKey,
                },
                body: JSON.stringify(payload),
            });

            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            return true;
        } catch (err) {
            this.failed = true;
            console.error('[WebChat] publish failed:', err);
            return false;
        }
    }

    publishStatus(status) {
        return this.publish({ type: 'status', status });
    }

    publishProvisional({ source = '', translation = '', speaker = null, language = null }) {
        return this.publish({
            type: 'provisional',
            source,
            translation,
            speaker,
            language,
        });
    }

    publishFinal({ source = '', translation = '', speaker = null, language = null }) {
        return this.publish({
            type: 'final',
            source,
            translation,
            speaker,
            language,
        });
    }

    reset() {
        this.roomId = null;
        this.shareUrl = '';
        this.seq = 1;
        this.failed = false;
    }
}

export const webChatPublisher = new WebChatPublisher();
