from django.urls import path, include
from rest_framework.routers import DefaultRouter
from .views import ApplicationRequestViewSet

router = DefaultRouter()
router.register(r'application_request', ApplicationRequestViewSet)

urlpatterns = [
    path('', include(router.urls)),
]
