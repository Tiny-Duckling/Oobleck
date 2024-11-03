"""
URL configuration for backend project.

The `urlpatterns` list routes URLs to views. For more information please see:
    https://docs.djangoproject.com/en/5.0/topics/http/urls/
Examples:
Function views
    1. Add an import:  from my_app import views
    2. Add a URL to urlpatterns:  path('', views.home, name='home')
Class-based views
    1. Add an import:  from other_app.views import Home
    2. Add a URL to urlpatterns:  path('', Home.as_view(), name='home')
Including another URLconf
    1. Import the include() function: from django.urls import include, path
    2. Add a URL to urlpatterns:  path('blog/', include('blog.urls'))
"""
from allauth.account.views import ConfirmEmailView
from dj_rest_auth.views import PasswordResetConfirmView
from django.contrib import admin
from django.urls import path, include

urlpatterns = [
    path('admin/', admin.site.urls),
    # path('password-reset/confirm/<uidb64>[0-9A-Za-z_\-]+)/(?P<token>[0-9A-Za-z]{1,13}-[0-9A-Za-z]{1,32})/$',
    #     PasswordResetConfirmView.as_view(template_name="password_reset_confirm.html"),
    #     name='password_reset_confirm'),
    path('password/reset/confirm/<uidb64>/<token>', PasswordResetConfirmView.as_view(), name='password_reset_confirm'),
    path('', include('dj_rest_auth.urls')),
    path('register/account-confirm-email/<str:key>/', ConfirmEmailView.as_view()),
    path('register/', include('dj_rest_auth.registration.urls')),
    path('career/', include('jobs.urls')),
]
