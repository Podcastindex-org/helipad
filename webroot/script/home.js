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


    //Initialize the page
    initPage();

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
        var url = '/api/v1/boosts?index=' + boostIndex;
        if (max > 0) {
            url += '&count=' + max;
        }
        if (old) {
            url += '&old=true';
        }

        $.ajax({
            url: url,
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            success: function (data) {
                data.forEach((element, index) => {
                    let displayedMessageCount = $('div.outgoing_msg').length;
                    //console.log(element);
                    let boostMessage = element.message || "";
                    let boostSats = Math.trunc(element.value_msat_total / 1000) || Math.trunc(element.value_msat / 1000);
                    let boostActualSats = Math.trunc(element.value_msat / 1000) || 0;
                    let boostIndex = element.index;
                    let boostAction = element.action;
                    let boostSender = element.sender;
                    let boostApp = element.app;
                    let boostPodcast = element.podcast;
                    let boostEpisode = element.episode;
                    let boostRemotePodcast = element.remote_podcast;
                    let boostRemoteEpisode = element.remote_episode;
                    let boostTlv = null;

                    try {
                        boostTlv = JSON.parse(element.tlv)
                    }
                    catch {}

                    //Icon
                    let appIcon = appList[boostApp.toLowerCase()] || {};
                    let appIconUrl = appIconUrlBase + (appIcon.icon || 'unknown');
                    let appIconHref = appIcon.url || '#';

                    //Sender
                    if (boostSender.trim() != "") {
                        boostSender = 'from ' + boostSender;
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

                    if (!messageIds.includes(boostIndex) && element.action == 2) {
                        let dateTime = new Date(element.time * 1000).toISOString();
                        $('div.nodata').remove();

                        //Build the message element
                        elMessage = '' +
                            '<div class="outgoing_msg message" data-msgid="' + boostIndex + '">' +
                            '  <div class="sent_msg">' +
                            '    <div class="sent_withd_msg">' +
                            '      <span class="app"><a href="' + appIconHref + '"><img src="' + appIconUrl + '" title="' + boostApp + '" alt="' + boostApp + '"></a></span>' +
                            '      <h5 class="sats">' + boostDisplayAmount + ' ' + boostSender + ' ' + boostNumerology + '</small></h5>' +
                            '      <time class="time_date" datetime="' + dateTime + '" title="' + dateFormat(dateTime) + '">' + 
                            '        <a href="#" style="color: blue" data-toggle="modal" data-target="#boostInfo">' + prettyDate(dateTime) + '</a>' + 
                            '      </time>' +
                            '      <small class="podcast_episode">' +
                            '        ' + boostPodcast + ' - ' + boostEpisode +
                            '        <span class="remote_item">' + boostRemoteInfo + '</span>' +
                            '      </small>' +
                            boostMessage
                        '    </div>' +
                        '  </div>' +
                        '</div>';

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
                                shootConfetti(1500);
                            }

                        }

                        //Update the tracking array
                        messageIds.push(boostIndex);
                        messageIds = messageIds.sort((a, b) => a - b);

                        if (shouldPew) {
                            //Pew pew pew!
                            pewAudio.play();
                        }
                    }
                });

                //Show a message if still building
                if ($('div.outgoing_msg').length == 0 && $('div.nodata').length == 0) {
                    inbox.prepend('<div class="nodata"><p>No data to show yet. Building the initial database may take some time if you have many ' +
                        'transactions, or maybe you have not been sent any boostagrams yet?</p>' +
                        '<p>This screen will automatically refresh as boostagrams are sent to you.</p>' +
                        '<p><a href="https://podcastindex.org/apps">Check out a Podcasting 2.0 app to send boosts and boostagrams.</a></p>' +
                        '<div class="lds-dual-ring"></div> Looking for boosts: <span class="invindex">' + currentInvoiceIndex + '</span>' +
                        '</div>');
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
                    inbox.append('<div class="loadmore"><a href="#">Show older boosts...</a></div>');
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
            url: "/api/v1/index",
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
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
        const $dialog = $(`
        <div id="boostInfo" class="modal" tabindex="-1">
          <div class="modal-dialog modal-lg modal-dialog-centered">
            <div class="modal-content">
              <div class="modal-header">
                <h5 class="modal-title">Boost Info</h5>
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

            $table.html('Loading boost...');

            $.getJSON(`/api/v1/boosts?index=${msgid}&count=1&old=true`, (result) => {
                if (!result[0]) {
                    return $table.html('Boost not found!');
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

    //Build the UI with the page loads
    async function initPage() {
        //Get starting balance and index number
        getBalance(true);
        await getAppList();
        await getNumerologyList();
        renderBoostInfo();
        getIndex();
    }

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
