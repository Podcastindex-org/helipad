$(document).ready(function () {
    let messages = $('div.mesgs');
    let inbox = messages.find('div.msg_history');
    let appIconUrlBase = '/image?name=';
    let pewAudioFile = '/pew.mp3';
    let pewAudio = new Audio(pewAudioFile);
    const urlParams = new URLSearchParams(window.location.search);
    const chat_id = urlParams.get('cid');
    var intvlChatPolling = null;
    var connection = null;
    var messageIds = [];

    getBoosts(null, 20, true, true);

    function getBoosts(startIndex, max, scrollToTop, old) {
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
        if(typeof boostIndex !== "number") {
            noIndex = true;
        }
        if(startIndex === null) {
            boostIndex = "";
        }
        if (typeof max !== "number") {
            max = 0;
        }
        if (typeof scrollToTop !== "boolean") {
            scrollToTop = true;
        }

        //Build the endpoint url
        var url = '/boosts?index=' + boostIndex;
        if(max > 0) {
            url += '&count=' + max;
        }
        if(old) {
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
                    let boostIndex = element.index;
                    let boostAction = element.action;
                    let boostSender = element.sender;
                    let boostApp = element.app;
                    let boostPodcast = element.podcast;
                    let boostEpisode = element.episode;

                    //Icon
                    var appIconUrl = "";
                    switch (boostApp.toLowerCase()) {
                        case 'fountain':
                            appIconUrl = appIconUrlBase + 'fountain';
                            break;
                        case 'podfriend':
                            appIconUrl = appIconUrlBase + 'podfriend';
                            break;
                        case 'castamatic':
                            appIconUrl = appIconUrlBase + 'castamatic';
                            break;
                        case 'curiocaster':
                            appIconUrl = appIconUrlBase + 'curiocaster';
                            break;
                        case 'breez':
                            appIconUrl = appIconUrlBase + 'breez';
                            break;
                        case 'podstation':
                            appIconUrl = appIconUrlBase + 'podstation';
                            break;
                        case 'sphinx':
                            appIconUrl = appIconUrlBase + 'sphinxchat';
                            break;
                        case 'podverse':
                            appIconUrl = appIconUrlBase + 'podverse';
                            break;
                        case 'zion':
                            appIconUrl = appIconUrlBase + 'zion';
                            break;

                    }

                    if (!messageIds.includes(boostIndex) && element.action == 2) {
                        let dateTime = new Date(element.time * 1000).toISOString();
                        $('div.nodata').remove();

                        //Build the message element
                        elMessage = '' +
                            '<div class="outgoing_msg message" data-msgid="' + boostIndex + '">' +
                            '  <div class="sent_msg">' +
                            '    <div class="sent_withd_msg">' +
                            '      <span class="app"><img src="' + appIconUrl + '"></span>' +
                            '      <h5>' + boostSats + ' sats <small>from ' + boostSender + '</small></h5>' +
                            '      <span class="time_date" data-timestamp="' + dateTime + '">' + prettyDate(dateTime) + '</span>' +
                            '      <small class="podcast_episode">' + boostPodcast + ' - ' + boostEpisode + '</small>' +
                            '      <br>' +
                            '      <hr>' +
                            '      <p>' + boostMessage + '</p>' +
                            '    </div>' +
                            '  </div>' +
                            '</div>';

                        //Insert the message in the right spot
                        if(displayedMessageCount == 0) {
                            inbox.prepend(elMessage);
                            //Scroll the list back up if necessary
                            if (scrollToTop) {
                                inbox.scrollTop();
                            }
                        } else {
                            //Get the closest matching id
                            var prepend = false;
                            let closestId = closest(messageIds, boostIndex);
                            if(boostIndex < closestId) {
                                prepend = true;
                            }

                            if(prepend) {
                                $('div.outgoing_msg[data-msgid='+closestId+']').after(elMessage);
                            } else {
                                $('div.outgoing_msg[data-msgid='+closestId+']').before(elMessage);
                            }

                        }

                        //Update the tracking array
                        messageIds.push(boostIndex);
                        messageIds = messageIds.sort((a,b) => a-b);

                        //Pew pew pew!
                        pewAudio.play();
                    }
                });

                //Show a message if still building
                if ($('div.outgoing_msg').length == 0 && $('div.nodata').length == 0) {
                    inbox.prepend('<div class="nodata">No data to show yet. Building the initial database may take some time. Check back in a few minutes...</div>');
                }

                //Load more link
                if ($('div.outgoing_msg').length > 0 && $('div.loadmore').length == 0 && ( boostIndex > 1 || noIndex)) {
                    inbox.append('<div class="loadmore"><a href="#">Show older boosts...</a></div>');
                }
            }
        });


    }

    //Load more messages handler
    $(document).on('click', 'div.loadmore a', function () {
        var old = true;
        let boostIndex = $('div.outgoing_msg:last').data('msgid');
        if (typeof boostIndex === "undefined") {
            return false;
        }

        boostIndex = boostIndex;
        if(boostIndex < 1) {
            boostIndex = 1;
            max = boostIndex
            old = false;
        }

        getBoosts(boostIndex, 200, false, old);

        return false;
    });

    //Set a periodic checker for new boosts
    setInterval(function () {
        if ($('div.outgoing_msg').length === 0) {
            getBoosts(null, 100, true, true);
        } else {
            getBoosts(null, 20, true, false);
        }
    }, 7000);

});

const closest = (arr, num) => {
    return arr.reduce((acc, val) => {
        if(Math.abs(val - num) < Math.abs(acc)){
            return val - num;
        }else{
            return acc;
        }
    }, Infinity) + num;
}