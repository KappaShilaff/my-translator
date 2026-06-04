class WebChatPublisher {
    constructor() {
        this.enabled = false;
        this.apiUrl = '';
        this.apiKey = '';
        this.roomId = null;
        this.shareUrl = '';
        this.seq = 1;
        this.failed = false;
        this.generation = 0;
        this.pending = Promise.resolve(false);
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
        this.generation += 1;
        this.pending = Promise.resolve(false);

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

    hasSessionFor(config) {
        const apiUrl = (config.apiUrl || '').trim().replace(/\/+$/, '');
        const apiKey = (config.apiKey || '').trim();
        return Boolean(
            config.enabled &&
            this.enabled &&
            this.roomId &&
            this.apiUrl === apiUrl &&
            this.apiKey === apiKey
        );
    }

    async ensureSession(config) {
        if (this.hasSessionFor(config)) {
            this.failed = false;
            return {
                room_id: this.roomId,
                share_url: this.shareUrl,
            };
        }

        return this.createSession(config);
    }

    async publish(event) {
        if (!this.enabled || !this.roomId || this.failed) return false;

        const generation = this.generation;
        const apiUrl = this.apiUrl;
        const apiKey = this.apiKey;
        const roomId = this.roomId;
        const payload = {
            ...event,
            seq: event.seq || this.seq++,
            ts: event.ts || Date.now(),
        };

        const send = async () => {
            if (
                generation !== this.generation ||
                !this.enabled ||
                this.failed ||
                this.roomId !== roomId
            ) {
                return false;
            }

            try {
                const response = await fetch(`${apiUrl}/api/sessions/${roomId}/events`, {
                    method: 'POST',
                    headers: {
                        'content-type': 'application/json',
                        'x-api-key': apiKey,
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
        };

        this.pending = this.pending.catch(() => false).then(send);
        return this.pending;
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
        this.generation += 1;
        this.pending = Promise.resolve(false);
    }
}

export const webChatPublisher = new WebChatPublisher();
