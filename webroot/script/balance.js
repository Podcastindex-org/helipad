const Balance = {
    currentBalance: null,

    getBalance: function() {
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
                    $('div.balanceDisplay').html('<span class="error" title="Error getting balance.">Err</span>');
                    return;
                }

                this.setBalance(balance);
            },

        });
    },

    setBalance: function(balance) {
        if (balance === this.currentBalance) {
            return; // no change
        }

        // Display the balance
        $('div.balanceDisplay').text('Balance: ').append(
            $('<span class="balance"></span>').text(numberFormat(balance))
        );

        // If the balance went up, do some fun stuff
        if (this.currentBalance && balance > this.currentBalance) {
            $('div.balanceDisplay').addClass('bump').on('animationend', () => {
                $('div.balanceDisplay').removeClass('bump');
            });
        }

        // This is now the current balance
        this.currentBalance = balance;
    }
}

$(document).ready(function () {
    Balance.getBalance();

    setInterval(() => {
        Balance.getBalance();
    }, 7000);
});
