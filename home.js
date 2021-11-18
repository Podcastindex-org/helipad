$(document).ready(function () {
    let messages = $('div.mesgs');
    let inbox = messages.find('div.msg_history');
    let appIconUrlBase = 'https://podcastindex.org/api/images/';
    let pewAudioFile = '/pew.mp3';
    let pewAudio = new Audio(pewAudioFile);
    const urlParams = new URLSearchParams(window.location.search);
    const chat_id = urlParams.get('cid');
    var intvlChatPolling = null;
    var connection = null;

    getBoosts();

    setInterval(function() {
        getBoosts();
    }, 7000);

    function getBoosts() {
        //Find newest index
        let lastIndex = $('div.outgoing_msg:first').data('msgid');
        if (typeof lastIndex === "undefined") {
            lastIndex = "";
        }

        $.ajax({
            url: '/boosts?index='+lastIndex,
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            success: function (data) {
                data.forEach((element, index) => {
                    console.log(element);
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
                            appIconUrl = appIconUrlBase + 'fountain.png';
                            break;
                        case 'podfriend':
                            appIconUrl = appIconUrlBase + 'podfriend.jpg';
                            break;
                        case 'castamatic':
                            appIconUrl = appIconUrlBase + 'castamatic.png';
                            break;
                        case 'curiocaster':
                            appIconUrl = appIconUrlBase + 'curiocaster.png';
                            break;
                        case 'breez':
                            appIconUrl = appIconUrlBase + 'breez.png';
                            break;
                        case 'podstation':
                            appIconUrl = appIconUrlBase + 'podstation.jpg';
                            break;
                        case 'sphinx':
                            appIconUrl = appIconUrlBase + 'sphinxchat.png';
                            break;
                        case 'podverse':
                            appIconUrl = appIconUrlBase + 'podverse.jpg';
                            break;
                    }

                    if(boostIndex > lastIndex && element.action == 2) {
                        let dateTime = new Date(element.time * 1000).toISOString();
                        $('div.nodata').remove();
                        inbox.prepend('' +
                            '<div class="outgoing_msg message" data-msgid="' + boostIndex + '">' +
                            '  <div class="sent_msg">' +
                            '    <div class="sent_withd_msg">' +
                            '      <span class="app"><img src="'+appIconUrl+'"></span>' +
                            '      <h5>' + boostSats + ' sats <small>from '+boostSender+'</small></h5>' +
                            '      <span class="time_date" data-timestamp="'+dateTime+'">' + prettyDate(dateTime) + '</span>' +
                            '      <small class="podcast_episode">'+boostPodcast+' - '+boostEpisode+'</small>' +
                            '      <br>' +
                            '      <hr>' +
                            '      <p>' + boostMessage + '</p>' +
                            '    </div>' +
                            '  </div>' +
                            '</div>');
                        inbox.animate({scrollTop: 0});
                        pewAudio.play();
                    }
                });
            }
        });

        //Show a message if still building
        if( $('div.outgoing_msg').length == 0 && $('div.nodata').length == 0 ) {
            inbox.prepend('<div class="nodata">No data to show yet. Building the initial database may take some time...</div>');
        }

    }

});