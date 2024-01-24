$(document).ready(function () {
    let messages = $('div.mesgs');
    let inbox = messages.find('div.msg_history');
    let appIconUrlBase = '/image?name=';
    let pewAudioFile = '/pew.mp3';
    let pewAudio = new Audio(pewAudioFile);
    let appList = {};
    let numerologyList = [];
    var connection = null;
    var messageIds = [];
    var currentInvoiceIndex = null;
    var currentBalance = null;
    var currentBalanceAmount = 0;

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
        // console.log(messageIds);

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

                    try {
                        boostTlv = JSON.parse(element.tlv)
                        boostReplyAddress = boostTlv.reply_address;
                        boostReplyCustomKey = boostTlv.reply_custom_key || '';
                        boostReplyCustomValue = boostTlv.reply_custom_value || '';
                    }
                    catch {}

                    //Icon
                    let appIcon = appList[boostApp.toLowerCase()] || {};
                    let appIconUrl = appIconUrlBase + (appIcon.icon || 'unknown');
                    let appIconHref = appIcon.url || '#';

                    //Person
                    let boostPerson = "";
                    if (config.pluralName == 'sent boosts' && boostTlv.name) {
                        boostPerson = `sent to ${boostTlv.name}`;
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

                    //If there is a difference between actual and stated sats, display it
                    var boostDisplayAmount = numberFormat(boostSats) + " sats";
                    if ((boostSats != boostActualSats) && boostSats > 0 && boostActualSats > 0) {
                        boostDisplayAmount = '<span class="more_info" title="' + numberFormat(boostActualSats) + ' sats received after splits/fees.">' + boostDisplayAmount + '</span>';
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
                                ${boostDisplayAmount} ${boostPerson} ${boostNumerology}
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

                        if (shouldPew && config.effects) {
                            //Pew pew pew!
                            pewAudio.play();
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

                var bcount = $('div.outgoing_msg:first').data('msgid') - $('div.outgoing_msg:last').data('msgid');
                if (typeof bcount !== "number") {
                    bcount = 9999;
                }

                //Update the csv export link
                var csvindex = $('div.outgoing_msg:first').data('msgid');
                if (typeof csvindex !== "number") {
                    csvindex = currentInvoiceIndex;
                }

                var endex = csvindex - bcount;
                $('span.csv a').attr('href', '/csv?index=' + csvindex + '&count=' + bcount + '&old=true' + '&end=' + endex);

                //Load more link
                if ($('div.outgoing_msg').length > 0 && $('div.loadmore').length == 0 && (boostIndex > 1 || noIndex)) {
                    inbox.append('<div class="loadmore"><a href="#">Show older ' + config.pluralName + '...</a></div>');
                }
            }
        });
    }

    //Determine any meaning behind this sat value
    //(uses boostbot numerology by default: https://github.com/valcanobacon/BoostBots)
    function gatherNumerology(value) {
        let numerology = value.toString();
        let meaning = [];

        // replace numerology with emojis
        numerologyList.forEach(item => {
            newNumerology = numerology.replaceAll(new RegExp(item.regex, 'g'), item.emoji);

            if (newNumerology != numerology) {
                meaning.push(item.name);
            }

            numerology = newNumerology;
        });

        // remove unmatched numbers
        numerology = numerology.replaceAll(new RegExp('[0-9]+', 'g'), '');

        // show meaning in mouse hover
        if (meaning) {
            numerology = '<span class="more_info" title="' + meaning.join(', ') + '">' + numerology + '</span>';
        }

        return numerology;
    }

    //Animate some confetti on the page with a given duration interval in milliseconds
    function shootConfetti(time) {
        startConfetti();
        setTimeout(function () {
            stopConfetti();
        }, time);
    }

    //Get the current channel balance from the node
    function getBalance(init) {
        //Get the current boost index number
        $.ajax({
            url: "/api/v1/balance",
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            error: function (xhr) {
                if (xhr.status === 403) {
                    window.location.href = "/login";
                }
            },
            success: function (data) {
                newBalance = data;
                //If the data returned wasn't a number then give an error
                if (typeof newBalance !== "number") {
                    $('div.balanceDisplay').html('<span title="Error getting balance." class="error">Err</span>');
                } else {
                    //Display the balance
                    $('div.balanceDisplay').html('<span class="balanceLabel">Balance: </span>' + numberFormat(newBalance));

                    //If the balance went up, do some fun stuff
                    if (newBalance > currentBalanceAmount && !init) {
                        $('div.balanceDisplay').addClass('bump');
                        setTimeout(function () {
                            $('div.balanceDisplay').removeClass('bump');
                        }, 1200);
                    }

                    //This is now the current balance
                    currentBalanceAmount = newBalance;
                }

            }
        });
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

                try {
                    tlv = JSON.parse(boost.tlv);
                }
                catch (e) {
                    return $table.html('Unable to parse TLV');
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

        $('#send-reply').click(function (event) {
            // validate form
            const $form = $('#reply-form');
            const valid = $form[0].checkValidity()

            $form.addClass('was-validated');

            if (!valid) {
                return;
            }

            // remember sender name for next time
            window.localStorage.setItem('lastSender', $('#sender-name').val());

            // send reply boost
            $('#send-reply').addClass('loading').prop('disabled', true);

            $.post(`/api/v1/reply`, $form.serialize(), function (result) {
                if (!result.success) {
                    $('#send-reply').removeClass('loading').prop('disabled', false);
                    return alert(result.message);
                }

                // mark boost as replied
                const { data: { payment_info: { reply_to_idx } } } = result;
                renderReplyButton(reply_to_idx, true);

                setTimeout(() => $dialog.modal('hide'), 1000);
            }).fail(function (req) {
                alert(req.responseText)
                $('#send-reply').removeClass('loading').prop('disabled', false);
            });
        });
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
        getBalance(true);
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
            initPage();
        } else {
            getBoosts(currentInvoiceIndex, 20, true, false, true);
            getBalance();
        }
    }, 7000);

    //Timestamp refresher
    setInterval(function () {
        updateTimestamps();
    }, 60000);

});
