$(document).ready(function () {
    let currentBalance = null;

    //Get the current channel balance from the node
    function getBalance() {
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
            success: function (balance) {
                // If the data returned wasn't a number then give an error
                if (typeof balance !== "number") {
                    $('div.balanceDisplay').html('<span class="error" title="Error getting balance.">Err</span>');
                    return;
                }

                if (balance === currentBalance) {
                    return; // no change
                }

                // Display the balance
                $('div.balanceDisplay').text('Balance: ').append(
                    $('<span class="balance"></span>').text(numberFormat(balance))
                );

                // If the balance went up, do some fun stuff
                if (currentBalance && balance > currentBalance) {
                    $('div.balanceDisplay').addClass('bump').on('animationend', () => {
                        $('div.balanceDisplay').removeClass('bump');
                    });
                }

                // This is now the current balance
                currentBalance = balance;
            }
        });
    }

    getBalance();

    setInterval(() => {
        getBalance();
    }, 7000);
});
