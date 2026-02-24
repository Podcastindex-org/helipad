class MidiPew {
    midiInitialized = false;
    midiAccess = null;
    midiOutput = null;

    async initMIDI() {
        if (this.midiInitialized || !navigator.requestMIDIAccess) {
            return;
        }

        try {
            this.midiAccess = await navigator.requestMIDIAccess();
            this.midiOutput = Array.from(this.midiAccess.outputs.values())[0];

            if (this.midiOutput) {
                console.log("MIDI output connected:", this.midiOutput.name);
            }
            this.midiInitialized = true;
        } catch (e) {
            console.warn("Failed to initialize MIDI:", e);
        }
    }

    async play(midi) {
        await this.initMIDI();

        if (!this.midiOutput) {
            console.warn("No MIDI output available");
            return;
        }

        const note = midi.note || 60;
        const velocity = midi.velocity || 100;
        const channel = (midi.channel || 1) - 1;
        const duration = midi.duration || 500;

        this.midiOutput.send([0x90 + channel, note, velocity]); // Note On

        return new Promise((resolve) => {
            setTimeout(() => {
                this.midiOutput.send([0x80 + channel, note, 0]); // Note Off
                console.log(`MIDI note ${note} sent on channel ${channel + 1} for ${duration}ms`);
                resolve();
            }, duration);
        });
    }
}

// Plays and queues the pews
class SoundPew {
    play(sound) {
        return new Promise((resolve, reject) => {
            const ts = new Date().getTime();
            const audio = new Audio(sound.sound_file + '?h=' + ts);
            audio.addEventListener('ended', resolve);
            audio.play().catch(reject);
        });
    }
}

// Plays and queues the pews
class TriggerQueue {
    sound = null;
    midi = null;
    playing = false;
    interval = null;
    queue = [];

    constructor() {
        this.sound = new SoundPew();
        this.midi = new MidiPew();
        this.playing = false;
        this.interval = null;
        this.queue = [];
        this.init();
    }

    init() {
        if (this.interval) return this;

        // work through the queue of pews
        this.interval = setInterval(() => {
            if (!this.playing && this.queue.length > 0) {
                const trigger = this.queue.shift();
                console.log('pewTriggers.handle', trigger);
                this.doTrigger(trigger);
            }
        }, 1000);

        return this;
    }

    // plays the requested pew sound
    doTrigger(trigger) {
        console.log('handleTrigger', trigger);
        if (!trigger) return;

        this.playing = true;

        const promises = [];

        if (trigger.midi) {
            promises.push(this.midi.play(trigger.midi));
        }

        if (trigger.sound && trigger.sound.sound_file) {
            promises.push(this.sound.play(trigger.sound));
        }

        Promise.all(promises).then(() => {
            this.playing = false;
        }).catch(err => {
            console.error('handleTrigger error', err);
            this.playing = false;
        });
    }

    // play or queue the sound
    handle(trigger) {
        console.log('handle', trigger);
        if (!this.playing && this.queue.length === 0) {
            this.doTrigger(trigger);
        }
        else {
            this.queue.push(trigger);
        }
    }

    // play the pew sound that corresponds with the donation amount
    handleTriggers(triggers) {
        if (triggers && triggers.length > 0) {
            triggers.forEach(trigger => {
                this.handle(trigger);
            });
        }
    }
}

class BalanceTracker {
    selector = 'div.balanceDisplay';
    currentBalance = null;
    interval = null;

    constructor(selector) {
        this.selector = selector || this.selector;
        this.getBalance();

        this.interval = setInterval(() => {
            this.getBalance();
        }, 7000);
    }

    getBalance() {
        //Get the current boost index number
        $.ajax({
            url: "/api/v1/balance",
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            error: (xhr) => {
                if (xhr.status === 403) {
                    window.location.href = "/login";
                }
            },
            success: (balance) => {
                // If the data returned wasn't a number then give an error
                if (typeof balance !== "number") {
                    $(this.selector).html('<span class="error" title="Error getting balance.">Err</span>');
                    return;
                }

                this.setBalance(balance);
            }
        });
    }

    setBalance(balance) {
        if (balance === this.currentBalance) {
            return; // no change
        }

        // Display the balance
        $(this.selector).text('Balance: ').append(
            $('<span class="balance"></span>').text(numberFormat(balance))
        );

        // If the balance went up, do some fun stuff
        if (this.currentBalance && balance > this.currentBalance) {
            $(this.selector).addClass('bump').on('animationend', () => {
                $(this.selector).removeClass('bump');
            });
        }

        // This is now the current balance
        this.currentBalance = balance;
    }
}

class HelipadWebsocket {
    ws = null;
    handler = null;
    reconnectDelay = 5000;

    constructor(handler) {
        this.handler = handler;
        this.connect();
    }

    connect() {
        this.ws = new WebSocket("/api/v1/ws");

        this.ws.onmessage = ({ data }) => {
            const [event, ...args] = JSON.parse(data);
            this.handler(event, args);
        };

        this.ws.onopen = () => console.log("WebSocket opened");

        this.ws.onclose = () => {
            console.log("WebSocket closed");
            setTimeout(() => this.connect(), this.reconnectDelay);
        };

        this.ws.onerror = (err) => console.error("WebSocket error", err);
    }

    close() {
        this.ws?.close();
    }
}