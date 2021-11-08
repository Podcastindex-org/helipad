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

    setInterval(function() {
        getBoosts();
    }, 5000);

    function getBoosts() {
        //Find newest index
        let lastIndex = $('div.outgoing_msg:last').data('msgid');
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

                    //Icon
                    var appIconUrl = "";
                    switch (element.app_name) {
                        case 'Fountain':
                            appIconUrl = appIconUrlBase + 'fountain.png';
                            break;
                        case 'Podfriend':
                            appIconUrl = appIconUrlBase + 'podfriend.jpg';
                            break;
                        case 'Castamatic':
                            appIconUrl = appIconUrlBase + 'castamatic.png';
                            break;
                        case 'Curiocaster':
                            appIconUrl = appIconUrlBase + 'curiocaster.png';
                            break;
                    }

                    let dateTime = new Date(element.time * 1000).toISOString();
                    inbox.append('<div class="outgoing_msg message" data-msgid="' + boostIndex + '">' +
                        '<div class="outgoing_msg_img"><img src=""></div>' +
                        '<div class="sent_msg"><div class="sent_withd_msg"><h5>' + boostSats + '</h5>' +
                        '<span class="time_date" data-timestamp="'+dateTime+'">' + prettyDate(dateTime) + '</span><p>' + boostMessage + '</p>' +
                        '</div></div></div>');
                    inbox.animate({scrollTop: 999999999});
                });
            }
        });
    }

});