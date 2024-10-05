$(document).ready(function () {
    let messages = $('div.mesgs');
    let inbox = messages.find('div.msg_history');
    let appIconUrlBase = 'image/';
    let pewSound = new PewSound();
    let appList = {};
    let numerologyList = [];
    var connection = null;
    var messageIds = [];
    var currentInvoiceIndex = null;
    var currentBalance = null;
    var currentBalanceAmount = 0;
    let nodeInfo = null;
    let settings = null;

    let config = {
        'listUrl': '/api/v1/boosts',
        'indexUrl': '/api/v1/index',
        'singularName': 'boost',
        'pluralName': 'boosts',
        'effects': true,
    }

    //Get a boost list starting at a particular invoice index
    function getBoosts(startIndex, max, scrollToTop, old, shouldPew) {
        var noIndex = false;

        //Find newest index
        let lastIndex = $('div.outgoing_msg:first').data('msgid');
        if (typeof lastIndex === "undefined") {
            lastIndex = "";
        }
        //console.log("Last index: ["+lastIndex+"]");
        let firstIndex = $('div.outgoing_msg:last').data('msgid');
        if (typeof firstIndex === "undefined") {
            firstIndex = "";
        }
        // console.log("First index: ["+firstIndex+"]");

        //Get current id set
        messageIds = [];
        $('div.outgoing_msg').map(function () {
            messageIds.push($(this).data('msgid'));
        });

        //Params
        if (typeof startIndex === "number") {
            boostIndex = startIndex;
        } else {
            boostIndex = lastIndex;
        }
        if (typeof boostIndex !== "number") {
            noIndex = true;
        }
        if (startIndex === null) {
            boostIndex = lastIndex + 20;
        }
        if (typeof max !== "number") {
            max = 0;
        }
        if (typeof scrollToTop !== "boolean") {
            scrollToTop = true;
        }

        //Override shouldPew for receiving our first boost
        if ($('div.nodata').length) {
            shouldPew = true;
        }

        //Build the endpoint url
        let params = {'index': boostIndex};

        if (max > 0) {
            params.count = max;
        }

        if (old) {
            params.old = true;
        }

        let url = config.listUrl + '?' + $.param(params);

        $.ajax({
            url: url,
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            error: function (xhr) {
                if (xhr.status === 403) {
                    window.location.href = "/login";
                }
            },
            success: function (data) {
                data.forEach((element, index) => {
                    let displayedMessageCount = $('div.outgoing_msg').length;
                    //console.log(element);
                    let boostMessage = element.message || "";
                    let boostSats = Math.trunc(element.value_msat_total / 1000) || Math.trunc(element.value_msat / 1000);
                    let boostActualSats = Math.trunc(element.value_msat / 1000) || 0;
                    let boostIndex = element.index;
                    let boostAction = element.action;
                    let boostApp = element.app;
                    let boostPodcast = element.podcast;
                    let boostEpisode = element.episode;
                    let boostRemotePodcast = element.remote_podcast;
                    let boostRemoteEpisode = element.remote_episode;
                    let boostReplySent = element.reply_sent;
                    let boostTlv = {};
                    let boostReplyAddress;
                    let boostReplyCustomKey;
                    let boostReplyCustomValue;

                    if (settings.hide_boosts && boostSats < settings.hide_boosts_below && config.pluralName != 'streams') {
                        return; // boost amount lower than minimum
                    }

                    try {
                        boostTlv = JSON.parse(element.tlv)
                        boostReplyAddress = boostTlv.reply_address;
                        boostReplyCustomKey = boostTlv.reply_custom_key || '';
                        boostReplyCustomValue = boostTlv.reply_custom_value || '';
                    }
                    catch {}

                    //Icon
                    let appIcon = appList[boostApp.toLowerCase()] || {};
                    let appIconUrl = appIconUrlBase + (appIcon.icon || 'unknown') + '.png';
                    let appIconHref = appIcon.url || '#';

                    //Person
                    let boostPerson = "";
                    if (config.pluralName == 'sent boosts' && boostTlv.name) {
                        boostPerson = `to ${boostTlv.name}`;
                    }
                    else if (element.sender.trim() != "") {
                        boostPerson = `from ${element.sender}`;
                    }

                    //Format the boost message
                    if (boostMessage.trim() != "") {
                        boostMessage = '' +
                            '      <hr>' +
                            '      <p>' + boostMessage + '</p>';
                    }

                    //Show sat amount and more info
                    let boostDisplayAmount = numberFormat(boostSats) + " sats";
                    let boostSentReceived = (config.pluralName == 'sent boosts' ? 'sent' : 'received');

                    if (settings.show_received_sats) {
                        boostDisplayAmount = `${numberFormat(boostActualSats)} sats ${boostSentReceived}`;
                    }

                    //If there is a difference between actual and stated sats, display it
                    if ((boostSats != boostActualSats) && boostSats > 0 && boostActualSats > 0) {
                        let boostMoreInfo = `${numberFormat(boostActualSats)} of ${numberFormat(boostSats)} sats ${boostSentReceived} after splits/fees.`;
                        boostDisplayAmount = '<span class="more_info" title="' + boostMoreInfo + '">' + boostDisplayAmount + '</span>';
                    }

                    let boostSplitPercentage = '';
                    if (settings.show_split_percentage) {
                        const split = Math.round(100 * boostActualSats / boostSats);

                        if (settings.show_received_sats) {
                            boostSplitPercentage = ' <small>(' + split + '% of ' + numberFormat(boostSats) + ' sats)</small>';
                        }
                        else {
                            boostSplitPercentage = ' <small>(' + split + '% split)</small>';
                        }
                    }

                    //Show clock icon for automated boosts
                    if (boostTlv && boostTlv.action == "auto") {
                        boostDisplayAmount = `
                        <span title="Automated boost">
                          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" fill="currentColor" class="bi bi-clock" viewBox="0 0 16 16" style="vertical-align: bottom;">
                            <path d="M8.515 1.019A7 7 0 0 0 8 1V0a8 8 0 0 1 .589.022l-.074.997zm2.004.45a7.003 7.003 0 0 0-.985-.299l.219-.976c.383.086.76.2 1.126.342l-.36.933zm1.37.71a7.01 7.01 0 0 0-.439-.27l.493-.87a8.025 8.025 0 0 1 .979.654l-.615.789a6.996 6.996 0 0 0-.418-.302zm1.834 1.79a6.99 6.99 0 0 0-.653-.796l.724-.69c.27.285.52.59.747.91l-.818.576zm.744 1.352a7.08 7.08 0 0 0-.214-.468l.893-.45a7.976 7.976 0 0 1 .45 1.088l-.95.313a7.023 7.023 0 0 0-.179-.483zm.53 2.507a6.991 6.991 0 0 0-.1-1.025l.985-.17c.067.386.106.778.116 1.17l-1 .025zm-.131 1.538c.033-.17.06-.339.081-.51l.993.123a7.957 7.957 0 0 1-.23 1.155l-.964-.267c.046-.165.086-.332.12-.501zm-.952 2.379c.184-.29.346-.594.486-.908l.914.405c-.16.36-.345.706-.555 1.038l-.845-.535zm-.964 1.205c.122-.122.239-.248.35-.378l.758.653a8.073 8.073 0 0 1-.401.432l-.707-.707z"/>
                            <path d="M8 1a7 7 0 1 0 4.95 11.95l.707.707A8.001 8.001 0 1 1 8 0v1z"/>
                            <path d="M7.5 3a.5.5 0 0 1 .5.5v5.21l3.248 1.856a.5.5 0 0 1-.496.868l-3.5-2A.5.5 0 0 1 7 9V3.5a.5.5 0 0 1 .5-.5z"/>
                          </svg>
                        </span>` + boostDisplayAmount;
                    }

                    //Determine the numerology behind the sat amount
                    boostNumerology = gatherNumerology(boostSats);

                    //Generate remote item and link to podcastindex website if one exists
                    let boostRemoteInfo = '';
                    if (boostRemoteEpisode) {
                        boostRemoteInfo = '(' + boostRemotePodcast + ' - ' + boostRemoteEpisode + ')';

                        if (boostTlv && boostTlv.remote_feed_guid) {
                            boostRemoteInfo = `
                            <a href="https://podcastindex.org/podcast/${boostTlv.remote_feed_guid}" target="_blank" style="color: blue;">
                                ${boostRemoteInfo}
                            </a>`;
                        }
                    }

                    if (!messageIds.includes(boostIndex)) {
                        let dateTime = new Date(element.time * 1000).toISOString();
                        $('div.nodata').remove();

                        //Build the message element
                        elMessage = `
                        <div class="outgoing_msg message" data-msgid="${boostIndex}" style="width: 100%">
                          <div class="sent_msg">
                            <div class="sent_withd_msg">
                              <span class="app">
                                <a href="${appIconHref}"><img src="${appIconUrl}" title="${boostApp}" alt="${boostApp}"></a>
                              </span>
                              <div class="pull-right text-right">
                                <time class="time_date" datetime="${dateTime}" title="${dateFormat(dateTime)}">
                                  <a href="#" style="color: blue" data-toggle="modal" data-target="#boostInfo">
                                    ${prettyDate(dateTime)}
                                  </a>
                                </time>
                                <div class="reply-to-boost-div">
                                </div>
                              </div>
                              <h5 class="sats">
                                ${boostDisplayAmount} ${boostPerson} ${boostNumerology} ${boostSplitPercentage}
                              </h5>
                              <small class="podcast_episode">
                                ${boostPodcast} - ${boostEpisode}
                                <span class="remote_item">${boostRemoteInfo}</span>
                              </small>
                              <div style="clear: both">
                                ${boostMessage}
                              </div>
                            </div>
                          </div>
                        </div>`;

                        //Insert the message in the right spot
                        if (displayedMessageCount == 0) {
                            inbox.prepend(elMessage);
                            //Scroll the list back up if necessary
                            if (scrollToTop) {
                                inbox.scrollTop();
                            }
                        } else {
                            //Get the closest matching id
                            var prepend = false;
                            let closestId = closest(messageIds, boostIndex);
                            if (boostIndex < closestId) {
                                prepend = true;
                            }

                            if (prepend) {
                                $('div.outgoing_msg[data-msgid=' + closestId + ']').after(elMessage);

                            } else {
                                $('div.outgoing_msg[data-msgid=' + closestId + ']').before(elMessage);

                                if (config.effects) {
                                    shootConfetti(1500);
                                }
                            }

                        }

                        // Show reply button for boosts/streams if reply address received
                        if (boostReplyAddress && (config.pluralName == 'boosts' || config.pluralName == 'streams')) {
                            // Attach reply data to the message
                            $('div.outgoing_msg[data-msgid=' + boostIndex + ']').data({
                                'index': boostIndex,
                                'podcast': boostPodcast,
                                'episode': boostEpisode,
                                'replyAddress': boostReplyAddress,
                                'replySender': element.sender,
                                'replyCustomKey': boostReplyCustomKey,
                                'replyCustomValue': boostReplyCustomValue,
                            });

                            renderReplyButton(boostIndex, boostReplySent);
                        }

                        //Update the tracking array
                        messageIds.push(boostIndex);
                        messageIds = messageIds.sort((a, b) => a - b);

                        if (shouldPew && config.effects && settings.play_pew) {
                            //Pew pew pew!
                            playPew(boostSats);
                        }
                    }
                });

                //Show a message if still building
                if ($('div.outgoing_msg').length == 0 && $('div.nodata').length == 0) {
                    if (config.pluralName == 'boosts') {
                        inbox.prepend('<div class="nodata"><p>No data to show yet. Building the initial database may take some time if you have many ' +
                            'transactions, or maybe you have not been sent any boostagrams yet?</p>' +
                            '<p>This screen will automatically refresh as boostagrams are sent to you.</p>' +
                            '<p><a href="https://podcastindex.org/apps">Check out a Podcasting 2.0 app to send boosts and boostagrams.</a></p>' +
                            '<div class="lds-dual-ring"></div> Looking for boosts: <span class="invindex">' + currentInvoiceIndex + '</span>' +
                            '</div>');
                    }
                    else if (config.pluralName == 'streams') {
                        inbox.prepend('<div class="nodata"><p>No data to show yet. Building the initial database may take some time if you have many ' +
                            'transactions, or maybe you have not had any satoshis streamed to you yet?</p>' +
                            '<p>This screen will automatically refresh as satoshis are streamed to you.</p>' +
                            '<p><a href="https://podcastindex.org/apps">Check out a Podcasting 2.0 app to stream satoshis.</a></p>' +
                            '<div class="lds-dual-ring"></div> Looking for streams: <span class="invindex">' + currentInvoiceIndex + '</span>' +
                            '</div>');
                    }
                    else if (config.pluralName == 'sent boosts') {
                        inbox.prepend('<div class="nodata"><p>No data to show yet. Building the initial database may take some time if you have many ' +
                            'transactions, or maybe you have not sent any satoshis from your node yet?</p>' +
                            '<p>This screen will automatically refresh as you send satoshis from your node.</p>' +
                            '<div class="lds-dual-ring"></div> Looking for sent boosts: <span class="invindex">' + currentInvoiceIndex + '</span>' +
                            '</div>');
                    }
                }
                $('div.nodata span.invindex').text(currentInvoiceIndex);

                let list = config.pluralName;
                if (config.pluralName == 'sent boosts') {
                    list = 'sent';
                }

                let bcount = $('div.outgoing_msg').length;
                if (typeof bcount !== "number") {
                    bcount = 9999;
                }

                //Update the csv export link
                let csvindex = $('div.outgoing_msg:first').data('msgid');
                if (typeof csvindex !== "number") {
                    csvindex = currentInvoiceIndex;
                }

                let endex = $('div.outgoing_msg:last').data('msgid');
                if (typeof csvindex !== "number") {
                    endex = 1;
                }

                $('span.csv a').attr('href', '/csv?list=' + list + '&index=' + csvindex + '&count=' + bcount + '&old=true' + '&end=' + endex);

                //Load more link
                if ($('div.outgoing_msg').length > 0 && $('div.loadmore').length == 0 && (boostIndex > 1 || noIndex)) {
                    inbox.append('<div class="loadmore"><a href="#">Show older ' + config.pluralName + '...</a></div>');
                }
            }
        });
    }

    //Determine any meaning behind this sat value
    function parseNumerology(value) {
        const numerology = []
        const substitutions = []

        let textValue = value.toString()

        // loop through numerology and capture matches
        numerologyList.forEach(item => {
            if (item.equality == '<' && value < item.amount) {
                numerology.push(item)
            }
            else if (item.equality == '>=' && value >= item.amount) {
                numerology.push(item)
            }
            else if (item.equality == '=' && value == item.amount) {
                numerology.push(item)
                textValue = ''
            }
            else if (item.equality == '=~') {
                const regex = new RegExp(item.amount, 'g')

                for (let match of textValue.matchAll(regex)) {
                    // replace match with X's to prevent further matches
                    const replace = match[0].replaceAll(new RegExp('[0-9]', 'g'), 'X')
                    textValue = textValue.replace(match[0], replace)

                    // keep track of where this was matched in the value
                    substitutions[match.index] = item

                    // add gap to fill in later
                    numerology.push(null)
                }
            }
        })

        // fill in substitutions in order of where they appear in the value
        let subs = Object.values(substitutions)

        numerology.forEach((item, idx) => {
            numerology[idx] = item || subs.shift()
        })

        return numerology
    };

    //Get the emojis that correspond to the donation amount
    function gatherNumerology(value) {
        const matches = parseNumerology(value);
        const emojis = matches.map(num => num.emoji);
        const descriptions = matches.map(num => num.description);

        // show meaning in mouse hover
        if (emojis) {
            numerology = '<span class="more_info" title="' + descriptions.join(', ') + '">' + emojis.join('') + '</span>';
        }

        return numerology;
    }

    // Plays and queues the pews
    function PewSound() {
        this.audio = new Audio();
        this.playing = false;
        this.queue = [];

        // plays the requested pew sound
        const playSound = (src) => {
            this.playing = true;

            this.audio.src = src;
            this.audio.play();

            this.audio.addEventListener('ended', () => this.playing = false);
        }

        // work through the queue of pews
        setInterval(() => {
            if (!this.playing && this.queue.length > 0) {
                playSound(this.queue.shift());
            }
        }, 1000);

        // play or queue the sound
        this.play = (src) => {
            if (!this.playing && this.queue.length === 0) { // nothing playing or queued
                playSound(src); // play the sound
            }
            else {
                this.queue.push(src); // queue the sound
            }
        }
    }

    //Play the pew sound that corresponds with the donation amount
    function playPew(value) {
        // find the first pew with a sound file
        const pews = parseNumerology(value).filter(num => num.sound_file)
        let src = 'pew.mp3'; // default

        if (pews.length) {
            src = `sound/${pews[0].sound_file}`;
        }
        else if (settings.custom_pew_file) {
            src = `sound/${settings.custom_pew_file}`;
        }

        pewSound.play(src);
    }

    //Animate some confetti on the page with a given duration interval in milliseconds
    function shootConfetti(time) {
        startConfetti();
        setTimeout(function () {
            stopConfetti();
        }, time);
    }

    //Get the current node alias and pubkey
    async function getNodeInfo() {
        nodeInfo = await $.get(`/api/v1/node_info`);
    }

    //Get configured settings
    async function getSettings() {
        settings = await $.get(`/api/v1/settings`);
    }

    //Refresh the timestatmps of all the boosts on the list
    function updateTimestamps() {
        console.log("Updating timestamps...");
        $('time.time_date').each(function (_, el) {
            var $el = $(el);
            $el.find('a').text(prettyDate(new Date($el.attr('datetime'))));
        });
    }

    //Get the most recent invoice index the node knows about
    function getIndex() {
        //Get the current boost index number
        $.ajax({
            url: config.indexUrl,
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            error: function (xhr) {
                if (xhr.status === 403) {
                    window.location.href = "/login";
                }
            },
            success: function (data) {
                //console.log(data);
                currentInvoiceIndex = data;
                //console.log(typeof currentInvoiceIndex);
                if (typeof currentInvoiceIndex !== "number" || currentInvoiceIndex < 1) {
                    currentInvoiceIndex = 1;
                }
                getBoosts(currentInvoiceIndex, 100, true, true, false);
            }
        });
    }

    //Get the defined list of apps
    async function getAppList() {
        appList = await $.ajax({
            url: "/apps.json",
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json"
        });

        return appList;
    }

    //Get the defined numerology
    async function getNumerologyList() {
        numerologyList = await $.ajax({
            url: "/numerology.json",
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json"
        });

        return numerologyList;
    }

    //Render the boost info modal
    function renderBoostInfo() {
        if ($('#boostInfo').length) {
            return; // already added
        }

        const name = ucWords(config.singularName);
        const $dialog = $(`
        <div id="boostInfo" class="modal" tabindex="-1">
          <div class="modal-dialog modal-lg modal-dialog-centered">
            <div class="modal-content">
              <div class="modal-header">
                <h5 class="modal-title">${name} Info</h5>
                <button type="button" class="close" data-dismiss="modal" aria-label="Close">
                  <span aria-hidden="true">&times;</span>
                </button>
              </div>
              <div class="modal-body">
                <table class="table table-sm table-borderless">
                  <tbody></tbody>
                </table>
              </div>
              <div class="modal-footer">
                <div class="flex-fill">
                    <a id="download-tlv" href="#" target="_blank">Download TLV</a>
                </div>
                <button type="button" class="btn btn-secondary" data-dismiss="modal">Close</button>
              </div>
            </div>
          </div>
        </div>`).appendTo('body');

        $dialog.on('show.bs.modal', function (ev) {
            const $target = $(ev.relatedTarget);
            const msgid = $target.closest(".outgoing_msg").data('msgid');
            const $table = $dialog.find('.modal-body table tbody');

            $table.html(`Loading ${config.singularName}...`);

            $.getJSON(`${config.listUrl}?index=${msgid}&count=1&old=true`, (result) => {
                if (!result[0]) {
                    return $table.html(`${name} not found!`);
                }

                const boost = result[0];
                let tlv = null;

                $('#download-tlv')
                    .prop('href', 'data:application/json;charset=utf-8,' + encodeURIComponent(boost.tlv))
                    .prop('download', `tlv-${msgid}.json`)

                try {
                    tlv = JSON.parse(boost.tlv);
                }
                catch (e) {
                    $table.empty()
                    $table.append('<tr><td colspan="2">Unable to parse TLV</td></tr>')
                    $table.append(
                        $('<tr>').append($('<th>').text('tlv')).append($('<td>').text(boost.tlv))
                    )
                    return
                }

                $table.empty().append(
                    Object.keys(tlv).map((key) => (
                        $('<tr>').append($('<th>').text(key)).append($('<td>').text(tlv[key]))
                    ))
                );
            });
        });
    }

    function renderReplyModal() {
        if ($('#replyModal').length) {
            return; // already added
        }

        const $dialog = $(`
        <div id="replyModal" class="modal" tabindex="-1">
          <div class="modal-dialog modal-lg modal-dialog-centered">
            <div class="modal-content">
              <form id="reply-form" class="needs-validation">
                <div class="modal-header">
                  <h5 class="modal-title">Reply to Boost</h5>
                  <button type="button" class="close" data-dismiss="modal" aria-label="Close">
                    <span aria-hidden="true">&times;</span>
                  </button>
                </div>
                <div class="modal-body">
                  <div class="form-group row">
                    <label for="recipient-name" class="col-sm-2 col-form-label">Recipient:</label>
                    <div class="col-sm-10 col-form-label text-truncate">
                        <div id="recipient-name"></div>
                        <small id="recipient-addr" class="text-black-50"></small>
                    </div>
                  </div>
                  <div class="form-group row">
                    <label for="sender-name" class="col-sm-2 col-form-label">Sender:</label>
                    <div class="col-sm-10">
                      <input id="sender-name" name="sender" type="text" class="form-control" placeholder="anonymous">
                    </div>
                  </div>
                  <div class="form-group row">
                    <label for="sat-amt" class="col-sm-2 col-form-label">Sats:</label>
                    <div class="col-sm-10">
                      <input id="sat-amt" name="sats" type="number" class="form-control w-auto" placeholder="sats" min="1" required>
                      <div class="invalid-feedback">
                        Please enter the number of sats to send.
                      </div>
                    </div>
                  </div>
                  <div class="form-group row ${window.webln ? '' : 'd-none'}">
                    <label for="send-from" class="col-sm-2 col-form-label pt-0">Send from:</label>
                    <div class="col-sm-10">
                      <div class="form-check form-check-inline">
                        <input class="form-check-input" type="radio" name="send-from" id="send-from-node" value="node" checked required>
                        <label class="form-check-label" for="send-from-node">Node</label>
                      </div>
                      <div class="form-check form-check-inline">
                        <input class="form-check-input" type="radio" name="send-from" id="send-from-browser" value="browser" required>
                        <label class="form-check-label" for="send-from-browser">Browser</label>
                      </div>
                    </div>
                  </div>
                  <div class="form-group">
                    <label for="message-text" class="col-form-label">Message:</label>
                    <textarea id="message-text" name="message" class="form-control" style="height: 8em;" maxlength="500"></textarea>
                    <span id="message-chars">500</span> characters remaining
                  </div>
                </div>
                <div class="modal-footer">
                  <input id="reply-index" name="index" type="hidden" value="" required>
                  <button type="button" class="btn btn-secondary" data-dismiss="modal">Close</button>
                  <button id="send-reply" class="btn btn-primary" type="button">
                    <div class="send-reply-title">
                      <svg class="mr-1" xmlns="http://www.w3.org/2000/svg" height="1em" viewBox="0 0 512 512" fill="currentColor">
                        <!--! Font Awesome Free 6.4.0 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license (Commercial License) Copyright 2023 Fonticons, Inc. -->
                        <path d="M156.6 384.9L125.7 354c-8.5-8.5-11.5-20.8-7.7-32.2c3-8.9 7-20.5 11.8-33.8L24 288c-8.6 0-16.6-4.6-20.9-12.1s-4.2-16.7 .2-24.1l52.5-88.5c13-21.9 36.5-35.3 61.9-35.3l82.3 0c2.4-4 4.8-7.7 7.2-11.3C289.1-4.1 411.1-8.1 483.9 5.3c11.6 2.1 20.6 11.2 22.8 22.8c13.4 72.9 9.3 194.8-111.4 276.7c-3.5 2.4-7.3 4.8-11.3 7.2v82.3c0 25.4-13.4 49-35.3 61.9l-88.5 52.5c-7.4 4.4-16.6 4.5-24.1 .2s-12.1-12.2-12.1-20.9V380.8c-14.1 4.9-26.4 8.9-35.7 11.9c-11.2 3.6-23.4 .5-31.8-7.8zM384 168a40 40 0 1 0 0-80 40 40 0 1 0 0 80z"/>
                      </svg>
                      Boost
                    </div>
                    <div class="send-reply-loading">
                      <span class="spinner mr-1"></span>
                      Boosting
                    </div>
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>`).appendTo('body');

        $dialog.on('show.bs.modal', function (ev) {
            // reset form
            const data = $(ev.relatedTarget).closest('div[data-msgid]').data();
            $('#reply-index').val(data.index);

            $('#recipient-addr').empty();

            if (data.replySender) {
                $('#recipient-name').text(data.replySender);
                $('#recipient-addr').text(data.replyAddress);
            }
            else {
                $('#recipient-name').text(data.replyAddress);
            }

            $('#sender-name').val(window.localStorage.getItem('lastSender') || '');

            const lastWallet = window.localStorage.getItem('lastWallet');

            if (lastWallet && window.webln) {
                $('#send-from-node').prop('checked', lastWallet == 'node')
                $('#send-from-browser').prop('checked', lastWallet == 'browser')
            }
            else {
                $('#send-from-node').prop('checked', true);
                $('#send-from-browser').prop('checked', false);
            }

            $('#sat-amt, #message-text').val('');

            $('#message-chars').text(
                $('#message-text').prop('maxLength')
            );

            $('#send-reply').removeClass('loading').prop('disabled', false);
            $('#reply-form').removeClass('was-validated');
        });

        $('#message-text').on('change keydown keyup', function () {
            // update X characters remaining count
            $('#message-chars').text(this.maxLength - this.value.length);
        });

        $('#send-reply').click(async function (event) {
            // validate form
            const $form = $('#reply-form');
            const valid = $form[0].checkValidity()

            $form.addClass('was-validated');

            if (!valid) {
                return;
            }

            // remember sender name for next time
            window.localStorage.setItem('lastSender', $('#sender-name').val());
            window.localStorage.setItem('lastWallet', $form.find('input[name="send-from"]:checked').val());

            // send reply boost
            $('#send-reply').addClass('loading').prop('disabled', true);

            if ($('#send-from-browser:checked').length) { //browser boost
                const index = $form.find('#reply-index').val();
                const boost = $(`div[data-msgid=${index}]`).data();
                const reply = {
                    'sender' : $form.find('#sender-name').val(),
                    'sats'   : $form.find('#sat-amt').val(),
                    'message': $form.find('#message-text').val(),
                };

                try {
                    await sendBrowserReplyBoost(boost, reply);
                    await $.post(`/api/v1/mark_replied`, { 'index': index });
                    renderReplyButton(index, true);
                    setTimeout(() => $dialog.modal('hide'), 1000);
                }
                catch (err) {
                    alert(err.responseText || err);
                    $('#send-reply').removeClass('loading').prop('disabled', false);
                }
            }
            else { //node boost
                $.post(`/api/v1/reply`, $form.serialize()).then(result => {
                    if (!result.success) {
                        $('#send-reply').removeClass('loading').prop('disabled', false);
                        return alert(result.message);
                    }

                    // mark boost as replied
                    const { data: { payment_info: { reply_to_idx } } } = result;
                    renderReplyButton(reply_to_idx, true);

                    setTimeout(() => $dialog.modal('hide'), 1000);
                }).fail(req => {
                    alert(req.responseText);
                    $('#send-reply').removeClass('loading').prop('disabled', false);
                });
            }
        });
    }

    async function sendBrowserReplyBoost(boost, reply) {
        const params = {
            destination: '',
            amount: reply.sats,
            customRecords: {
                '7629169': JSON.stringify({
                    'action': 'boost',
                    'app_name': 'Helipad',
                    'app_version': $('#helipad-version').text(),
                    'podcast': boost.podcast,
                    'episode': boost.episode,
                    'reply_address': nodeInfo.node_pubkey,
                    'reply_custom_key': null,
                    'reply_custom_value': null,
                    'sender_name': reply.sender || 'Anonymous',
                    'message':  reply.message || '',
                    'value_msat': reply.sats * 1000,
                    'value_msat_total': reply.sats * 1000,
                }),
            },
        };

        if (boost.replyAddress.indexOf('@') !== -1) { // keysend address
            const keysendInfo = await resolveKeysendAddress(boost.replyAddress);

            params.destination = keysendInfo.pubkey;

            for (let data of keysendInfo.customData) {
                params.customRecords[data.customKey] = data.customValue;
            }
        }
        else { // normal pubkey/custom value
            params.destination = boost.replyAddress;

            if (boost.replyCustomKey && boost.replyCustomValue) {
                params.customRecords[boost.replyCustomKey] = boost.replyCustomValue;
            }
        }

        await window.webln.enable();

        return await window.webln.keysend(params);
    }

    function renderReplyButton(index, replySent) {
        let className = 'btn-outline-primary';
        let title = 'Reply';
        let icon = `<svg class="mr-1" xmlns="http://www.w3.org/2000/svg" height="1em" viewBox="0 0 512 512" fill="currentColor"><!--!Font Awesome Free 6.5.1 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license/free Copyright 2024 Fonticons, Inc.--><path d="M205 34.8c11.5 5.1 19 16.6 19 29.2v64H336c97.2 0 176 78.8 176 176c0 113.3-81.5 163.9-100.2 174.1c-2.5 1.4-5.3 1.9-8.1 1.9c-10.9 0-19.7-8.9-19.7-19.7c0-7.5 4.3-14.4 9.8-19.5c9.4-8.8 22.2-26.4 22.2-56.7c0-53-43-96-96-96H224v64c0 12.6-7.4 24.1-19 29.2s-25 3-34.4-5.4l-160-144C3.9 225.7 0 217.1 0 208s3.9-17.7 10.6-23.8l160-144c9.4-8.5 22.9-10.6 34.4-5.4z"/></svg>`;

        if (replySent) {
            className = 'btn-link text-primary';
            title = 'Replied';
            icon = `<svg class="mr-1" xmlns="http://www.w3.org/2000/svg" height="1em" viewBox="0 0 448 512" fill="currentColor"><!--!Font Awesome Free 6.5.1 by @fontawesome - https://fontawesome.com License - https://fontawesome.com/license/free Copyright 2024 Fonticons, Inc.--><path d="M438.6 105.4c12.5 12.5 12.5 32.8 0 45.3l-256 256c-12.5 12.5-32.8 12.5-45.3 0l-128-128c-12.5-12.5-12.5-32.8 0-45.3s32.8-12.5 45.3 0L160 338.7 393.4 105.4c12.5-12.5 32.8-12.5 45.3 0z"/></svg>`;
        }

        $('div.outgoing_msg[data-msgid=' + index + '] .reply-to-boost-div').html(`
            <a
                href="#"
                class="reply-to-boost btn btn-sm ${className} position-relative d-inline-flex align-items-center"
                data-toggle="modal"
                data-target="#replyModal"
            >
                ${icon} ${title}
            </a>
        `);
    }

    //Build the UI with the page loads
    async function initPage() {
        setConfig();
        renderReplyModal();
        //Get starting balance and index number
        await getNodeInfo();
        await getSettings();
        await getAppList();
        await getNumerologyList();
        renderBoostInfo();
        getIndex();
    }

    function setConfig() {
        const pathname = window.location.pathname;

        if (pathname == "/") {
            config.listUrl = '/api/v1/boosts';
            config.singularName = 'boost';
            config.pluralName = 'boosts';
        }
        else if (pathname == "/streams") {
            config.listUrl = '/api/v1/streams';
            config.singularName = 'stream';
            config.pluralName = 'streams';
        }
        else if (pathname == "/sent") {
            config.listUrl = '/api/v1/sent';
            config.indexUrl = '/api/v1/sent_index';
            config.singularName = 'sent boost';
            config.pluralName = 'sent boosts';
            config.effects = false;
        }
    }

    //Initialize the page
    initPage();

    //Load more messages handler
    $(document).on('click', 'div.loadmore a', function () {
        var old = true;
        let boostIndex = $('div.outgoing_msg:last').data('msgid');
        if (typeof boostIndex === "undefined") {
            return false;
        }

        boostIndex = boostIndex;
        if (boostIndex < 1) {
            boostIndex = 1;
            max = boostIndex
            old = false;
        }

        getBoosts(boostIndex, 100, false, old, false);

        return false;
    });

    //Boost and node info checker
    setInterval(async function () {
        if ($('div.outgoing_msg').length === 0) {
            getIndex();
        } else {
            getBoosts(currentInvoiceIndex, 20, true, false, true);
        }
    }, 7000);

    //Timestamp refresher
    setInterval(function () {
        updateTimestamps();
    }, 60000);

});
