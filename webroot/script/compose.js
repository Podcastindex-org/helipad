$(document).ready(function () {
    let messages = $('div.mesgs');
    let message = messages.find('div.msg_history');

    //Initialize the page
    //initPage();

    //Submit form
    function submit() {
        var sent;
        var form = $("#formId");
        var url = form.attr('action');
        $.ajax({
            type: "POST",
            url: url,
            data: form.serialize(),
            success: function(data) {
                alert("Boostagram sent successfully");
            },
            error: function(data) {
                alert("some Error");
            }
        });
        return sent;
    }
});
