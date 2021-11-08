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


    function getBoosts() {
        //Find newest index


        $.ajax({
            url: '/boosts',
            type: "GET",
            contentType: "application/json; charset=utf-8",
            dataType: "json",
            success: function (data) {
                data.forEach((element, index) => {
                    console.log(element);
                    let boostMessage = element.message || "";
                    let boostSats = Math.trunc(element.value_msat_total / 1000) || Math.trunc(element.value_msat / 1000);

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

                    inbox.append('<div class="incoming_msg"><div class="incoming_msg_img"><img src="'+appIconUrl+'"></div>' +
                        '<div class="received_msg"><div class="received_withd_msg"><h5 class="total_sats">'+boostSats+' sats</h5><p>' + boostMessage + '</p>'+
                        '<span class="time_date">' + prettyDate(element.time) + '</span></div></div></div>');
                });
            }
        });
    }

});